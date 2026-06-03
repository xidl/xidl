use xidl_parser::hir;

use super::contexts::{
    BodyParamContext, ClientMethodContext, ClientParamContext, CookieParamContext,
    HeaderParamContext, PathParamContext, QueryParamContext, RequestPayloadEntry, TsType,
};
use super::http::is_sequence_type;
use super::names::scoped_name;
use super::route_template::path_param_is_catch_all;
use super::type_expr::ts_type_for_type_spec;

#[derive(Clone)]
pub(crate) struct ParamInfo {
    pub(crate) name: String,
    pub(crate) raw_name: String,
    pub(crate) wire_name: String,
    pub(crate) ty: hir::TypeSpec,
    pub(crate) optional: bool,
    pub(crate) doc: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct MethodInfo {
    pub(crate) name: String,
    pub(crate) params: Vec<ParamInfo>,
    pub(crate) ret: ReturnType,
    pub(crate) response_name: Option<String>,
    pub(crate) http_method: String,
    pub(crate) path: String,
    pub(crate) request_name: Option<String>,
    pub(crate) request_schema_ref: Option<String>,
    pub(crate) path_params: Vec<ParamInfo>,
    pub(crate) query_params: Vec<ParamInfo>,
    pub(crate) header_params: Vec<ParamInfo>,
    pub(crate) cookie_params: Vec<ParamInfo>,
    pub(crate) body_params: Vec<ParamInfo>,
    pub(crate) output_params: Vec<ParamInfo>,
    pub(crate) is_server_stream: bool,
    pub(crate) is_client_stream: bool,
    pub(crate) doc: Vec<String>,
}

impl MethodInfo {
    pub(crate) fn to_template(&self, module_path: &[String]) -> ClientMethodContext {
        let params = self.client_params(module_path);
        let return_payload_ty = self.return_payload_ty(module_path);
        ClientMethodContext {
            name: self.name.clone(),
            params,
            return_ty: if self.is_server_stream {
                TsType::AsyncIterable(Box::new(return_payload_ty))
            } else {
                return_payload_ty
            },
            return_schema_ref: self.return_schema_ref(module_path),
            stream_item_ty: self.server_stream_item_ty(module_path),
            client_stream_item_ty: self.client_stream_item_ty(module_path),
            request_schema_ref: self.request_schema_ref.clone(),
            body_schema_ref: self.body_schema_ref(module_path),
            request_payload: self.request_payload(),
            path: self.path.clone(),
            http_method: self.http_method.clone(),
            path_params: self.path_params(),
            query_params: self.query_params(),
            header_params: self.header_params(),
            cookie_params: self.cookie_params(),
            body_params: self.body_params(),
            body_single: self.body_single(),
            is_server_stream: self.is_server_stream,
            is_client_stream: self.is_client_stream,
            doc: self.doc.clone(),
        }
    }

    fn return_schema_ref(&self, module_path: &[String]) -> Option<String> {
        if let Some(response_name) = &self.response_name {
            let full = scoped_name(module_path, response_name);
            Some(full) // Template will add 'zodSchemas.' and 'Schema'
        } else if self.ret.is_void {
            None
        } else {
            // Wait, zod_schema_for_type_spec now returns ZodSchema enum.
            // But this function returns Option<String> as a reference.
            // If it's an inline schema, it might be tricky.
            // Let's assume for now this is always a ref.
            None // TODO: handle inline schemas in client method
        }
    }

    fn body_schema_ref(&self, module_path: &[String]) -> Option<String> {
        if let Some(request_name) = &self.request_name {
            let full = scoped_name(module_path, request_name);
            Some(full)
        } else {
            None // TODO: handle inline body schemas
        }
    }

    fn client_params(&self, module_path: &[String]) -> Vec<ClientParamContext> {
        if self.is_client_stream {
            let item_ty = self
                .request_name
                .as_ref()
                .map(|name| TsType::ScopedName(scoped_name(module_path, name)))
                .unwrap_or(TsType::Void);
            return vec![ClientParamContext {
                name: "stream".to_string(),
                ty: TsType::AsyncIterable(Box::new(item_ty)),
            }];
        }
        self.params
            .iter()
            .map(|param| ClientParamContext {
                name: param.name.clone(),
                ty: {
                    let ty = ts_type_for_type_spec(&param.ty, module_path, TypeRefTarget::Client);
                    if param.optional {
                        TsType::Optional(Box::new(ty))
                    } else {
                        ty
                    }
                },
            })
            .collect()
    }

    fn return_payload_ty(&self, module_path: &[String]) -> TsType {
        if let Some(response_name) = &self.response_name {
            TsType::ScopedName(scoped_name(module_path, response_name))
        } else if self.ret.is_void {
            TsType::Void
        } else {
            ts_type_for_type_spec(
                self.ret.ty.as_ref().expect("return type"),
                module_path,
                TypeRefTarget::Client,
            )
        }
    }

    fn server_stream_item_ty(&self, module_path: &[String]) -> Option<TsType> {
        self.is_server_stream
            .then(|| self.return_payload_ty(module_path))
    }

    fn client_stream_item_ty(&self, module_path: &[String]) -> Option<TsType> {
        self.is_client_stream.then(|| {
            self.request_name
                .as_ref()
                .map(|name| TsType::ScopedName(scoped_name(module_path, name)))
                .unwrap_or(TsType::Void)
        })
    }

    fn request_payload(&self) -> Vec<RequestPayloadEntry> {
        self.params
            .iter()
            .map(|param| RequestPayloadEntry {
                raw_name: param.raw_name.clone(),
                name: param.name.clone(),
            })
            .collect()
    }

    fn path_params(&self) -> Vec<PathParamContext> {
        self.path_params
            .iter()
            .map(|param| PathParamContext {
                template_name: if path_param_is_catch_all(&self.path, &param.wire_name) {
                    format!("*{}", param.wire_name)
                } else {
                    param.wire_name.clone()
                },
                catch_all: path_param_is_catch_all(&self.path, &param.wire_name),
                access: parsed_access(self, &param.raw_name),
            })
            .collect()
    }

    fn query_params(&self) -> Vec<QueryParamContext> {
        self.query_params
            .iter()
            .map(|param| QueryParamContext {
                raw_name: param.wire_name.clone(),
                access: parsed_access(self, &param.raw_name),
            })
            .collect()
    }

    fn header_params(&self) -> Vec<HeaderParamContext> {
        self.header_params
            .iter()
            .map(|param| HeaderParamContext {
                raw_name: param.wire_name.clone(),
                access: parsed_access(self, &param.raw_name),
                is_multi: is_sequence_type(&param.ty),
            })
            .collect()
    }

    fn cookie_params(&self) -> Vec<CookieParamContext> {
        self.cookie_params
            .iter()
            .map(|param| CookieParamContext {
                raw_name: param.wire_name.clone(),
                access: parsed_access(self, &param.raw_name),
                is_multi: is_sequence_type(&param.ty),
            })
            .collect()
    }

    fn body_params(&self) -> Vec<BodyParamContext> {
        self.body_params
            .iter()
            .map(|param| BodyParamContext {
                raw_name: param.raw_name.clone(),
                access: parsed_access(self, &param.raw_name),
            })
            .collect()
    }

    fn body_single(&self) -> Option<BodyParamContext> {
        self.body_params.first().map(|param| BodyParamContext {
            raw_name: param.raw_name.clone(),
            access: parsed_access(self, &param.raw_name),
        })
    }
}

fn parsed_access(method: &MethodInfo, raw_name: &str) -> String {
    if method.request_schema_ref.is_some() {
        format!("parsed[\"{raw_name}\"]")
    } else {
        super::names::ts_ident(raw_name)
    }
}

#[derive(Clone)]
pub(crate) struct ReturnType {
    pub(crate) is_void: bool,
    pub(crate) ty: Option<hir::TypeSpec>,
}

impl ReturnType {
    pub(crate) fn void() -> Self {
        Self {
            is_void: true,
            ty: None,
        }
    }

    pub(crate) fn new(ty: hir::TypeSpec) -> Self {
        Self {
            is_void: false,
            ty: Some(ty),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum TypeRefTarget {
    Types,
    Client,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParamSource {
    Path,
    Query,
    Header,
    Cookie,
    Body,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParamDirection {
    In,
    Out,
    InOut,
}

pub(crate) struct RouteTemplate {
    pub(crate) path: String,
    pub(crate) path_params: std::collections::HashSet<String>,
    pub(crate) query_params: std::collections::HashSet<String>,
}
