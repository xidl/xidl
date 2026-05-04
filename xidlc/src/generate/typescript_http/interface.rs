use crate::error::{IdlcError, IdlcResult};
use crate::generate::typescript::TypescriptRenderer;
use crate::generate::typescript::definition::TypeRefTarget;
use crate::generate::typescript::definition::contexts::{
    ClientParamContext, ParamDeclContext, RequestContext, RequestZodContext,
};
use crate::generate::typescript::definition::names::{method_struct_prefix, scoped_name, ts_ident};
use crate::generate::typescript::definition::type_expr::{
    ts_type_for_type_spec, zod_schema_for_type_spec,
};
use serde::Serialize;
use xidl_parser::hir;
use xidl_parser::http_hir::{
    HttpHirDocument, HttpOperation, HttpParam, HttpParamKind,
    semantics::{HttpSecurityRequirement, HttpStreamCodec, HttpStreamKind},
};

#[derive(Default)]
pub(crate) struct TsHttpBlocks {
    pub(crate) types: Vec<String>,
    pub(crate) zod: Vec<String>,
    pub(crate) client: Vec<String>,
}

impl TsHttpBlocks {
    pub(crate) fn extend(&mut self, other: Self) {
        self.types.extend(other.types);
        self.zod.extend(other.zod);
        self.client.extend(other.client);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.types.is_empty() && self.zod.is_empty() && self.client.is_empty()
    }
}

#[derive(Serialize)]
struct ClientClassContext {
    client_name: String,
    methods: Vec<ClientMethodContext>,
}

#[derive(Serialize)]
struct ClientMethodContext {
    name: String,
    params: Vec<ClientParamContext>,
    return_ty: String,
    request_schema_ref: Option<String>,
    request_payload: Vec<RequestPayloadEntry>,
    path: String,
    http_method: String,
    request_content_type: String,
    response_content_type: String,
    path_params: Vec<PathParamContext>,
    query_params: Vec<ValueParamContext>,
    header_params: Vec<ValueParamContext>,
    cookie_params: Vec<ValueParamContext>,
    response_header_params: Vec<ValueParamContext>,
    response_cookie_params: Vec<ValueParamContext>,
    body_entries: Vec<RequestPayloadEntry>,
    body_single: Option<String>,
    response_schema_ref: Option<String>,
    response_body_mode: String,
    is_server_stream: bool,
    is_client_stream: bool,
    stream_item_ty: Option<String>,
    client_stream_item_ty: Option<String>,
    security: Vec<SecurityContext>,
}

#[derive(Clone, Serialize)]
struct RequestPayloadEntry {
    raw_name: String,
    access: String,
}

#[derive(Clone, Serialize)]
struct PathParamContext {
    template_name: String,
    access: String,
    catch_all: bool,
}

#[derive(Clone, Serialize)]
struct ValueParamContext {
    raw_name: String,
    access: String,
    optional: bool,
    is_multi: bool,
}

#[derive(Clone, Serialize)]
struct SecurityContext {
    kind: String,
    location: Option<String>,
    name: Option<String>,
    realm: Option<String>,
    scopes: Vec<String>,
}

pub(crate) fn render_interface(
    interface: &hir::InterfaceDcl,
    module_path: &[String],
    renderer: &TypescriptRenderer,
    http_hir: &HttpHirDocument,
) -> IdlcResult<TsHttpBlocks> {
    let hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
        return Ok(TsHttpBlocks::default());
    };
    let Some(http_interface) = http_hir.find_interface(module_path, &def.header.ident) else {
        return Ok(TsHttpBlocks::default());
    };
    let methods = http_interface
        .operations
        .iter()
        .map(|op| build_method_model(def.header.ident.as_str(), module_path, op))
        .collect::<IdlcResult<Vec<_>>>()?;
    let mut out = TsHttpBlocks::default();
    for method in &methods {
        render_request_types(&mut out, method, module_path, renderer)?;
        render_response_types(&mut out, method, module_path, renderer)?;
    }
    out.client.push(
        renderer.render_template(
            "http/client_class.ts.j2",
            &ClientClassContext {
                client_name: format!("{}Client", ts_ident(&def.header.ident)),
                methods: methods
                    .into_iter()
                    .map(MethodModel::into_client_context)
                    .collect(),
            },
        )?,
    );
    Ok(out)
}

struct MethodModel {
    name: String,
    params: Vec<ClientParamContext>,
    request_name: Option<String>,
    request_schema_ref: Option<String>,
    request_payload: Vec<RequestPayloadEntry>,
    response_name: Option<String>,
    response_schema_ref: Option<String>,
    request_content_type: String,
    response_content_type: String,
    path: String,
    http_method: String,
    path_params: Vec<PathParamContext>,
    query_params: Vec<ValueParamContext>,
    header_params: Vec<ValueParamContext>,
    cookie_params: Vec<ValueParamContext>,
    response_header_params: Vec<ValueParamContext>,
    response_cookie_params: Vec<ValueParamContext>,
    body_entries: Vec<RequestPayloadEntry>,
    body_single: Option<String>,
    return_ty: String,
    response_body_mode: String,
    stream_item_ty: Option<String>,
    client_stream_item_ty: Option<String>,
    is_server_stream: bool,
    is_client_stream: bool,
    security: Vec<SecurityContext>,
    request_fields: Vec<ParamDeclContext>,
    response_fields: Vec<ParamDeclContext>,
}

impl MethodModel {
    fn into_client_context(self) -> ClientMethodContext {
        ClientMethodContext {
            name: self.name,
            params: self.params,
            return_ty: self.return_ty,
            request_schema_ref: self.request_schema_ref,
            request_payload: self.request_payload,
            path: self.path,
            http_method: self.http_method,
            request_content_type: self.request_content_type,
            response_content_type: self.response_content_type,
            path_params: self.path_params,
            query_params: self.query_params,
            header_params: self.header_params,
            cookie_params: self.cookie_params,
            response_header_params: self.response_header_params,
            response_cookie_params: self.response_cookie_params,
            body_entries: self.body_entries,
            body_single: self.body_single,
            response_schema_ref: self.response_schema_ref,
            response_body_mode: self.response_body_mode,
            is_server_stream: self.is_server_stream,
            is_client_stream: self.is_client_stream,
            stream_item_ty: self.stream_item_ty,
            client_stream_item_ty: self.client_stream_item_ty,
            security: self.security,
        }
    }
}

fn build_method_model(
    interface_name: &str,
    module_path: &[String],
    op: &HttpOperation,
) -> IdlcResult<MethodModel> {
    validate_stream_support(op)?;
    let prefix = method_struct_prefix(interface_name, &op.name);
    let request_fields = op
        .request_params
        .iter()
        .map(|param| param_decl(param, module_path))
        .collect::<Vec<_>>();
    let response_fields = response_fields(op, module_path);
    let request_name = (!request_fields.is_empty()).then(|| format!("{prefix}Request"));
    let response_name = (!response_fields.is_empty() && !only_direct_return(op))
        .then(|| format!("{prefix}Response"));
    let request_schema_ref = request_name
        .as_ref()
        .filter(|_| !matches!(op.stream.kind, Some(HttpStreamKind::Client)))
        .map(|name| format!("ifaceSchemas.{}Schema", scoped_name(module_path, name)));
    let response_schema_ref = response_name
        .as_ref()
        .map(|name| format!("ifaceSchemas.{}Schema", scoped_name(module_path, name)));
    let return_ty = response_name
        .as_ref()
        .map(|name| scoped_name(module_path, name))
        .or_else(|| {
            op.return_type
                .as_ref()
                .map(|ty| ts_type_for_type_spec(ty, module_path, TypeRefTarget::Client))
        })
        .unwrap_or_else(|| "void".to_string());
    let client_stream_item_ty = client_stream_ty(op, module_path, &request_name);
    let path = op
        .routes
        .first()
        .map(|route| route.path.clone())
        .unwrap_or_else(|| "/".to_string());

    Ok(MethodModel {
        name: ts_ident(&op.name),
        params: client_params(op, module_path, &request_name),
        request_name,
        request_schema_ref,
        request_payload: op
            .request_params
            .iter()
            .map(|param| RequestPayloadEntry {
                raw_name: param.name.clone(),
                access: ts_ident(&param.name),
            })
            .collect(),
        response_name,
        response_schema_ref,
        request_content_type: request_content_type(op),
        response_content_type: response_content_type(op),
        path: path.clone(),
        http_method: http_method_name(op.method).to_string(),
        path_params: value_params(op, HttpParamKind::Path)
            .into_iter()
            .map(|param| PathParamContext {
                catch_all: path.contains(&format!("{{*{}}}", param.raw_name)),
                template_name: param.raw_name,
                access: param.access,
            })
            .collect(),
        query_params: value_params(op, HttpParamKind::Query),
        header_params: value_params(op, HttpParamKind::Header),
        cookie_params: value_params(op, HttpParamKind::Cookie),
        response_header_params: response_value_params(op, HttpParamKind::Header),
        response_cookie_params: response_value_params(op, HttpParamKind::Cookie),
        body_entries: op
            .request_params
            .iter()
            .filter(|param| matches!(param.kind, HttpParamKind::Body))
            .map(|param| RequestPayloadEntry {
                raw_name: param.name.clone(),
                access: ts_ident(&param.name),
            })
            .collect(),
        body_single: direct_body_access(&op.request_params),
        return_ty,
        response_body_mode: response_body_mode(op).to_string(),
        stream_item_ty: server_stream_ty(op, module_path),
        client_stream_item_ty,
        is_server_stream: matches!(op.stream.kind, Some(HttpStreamKind::Server)),
        is_client_stream: matches!(op.stream.kind, Some(HttpStreamKind::Client)),
        security: security_contexts(op),
        request_fields,
        response_fields,
    })
}

fn render_request_types(
    out: &mut TsHttpBlocks,
    method: &MethodModel,
    module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<()> {
    let Some(name) = method.request_name.as_ref() else {
        return Ok(());
    };
    out.types.push(renderer.render_template(
        "http/request.d.ts.j2",
        &RequestContext {
            name: name.to_string(),
            params: method.request_fields.clone(),
            doc: Vec::new(),
        },
    )?);
    out.zod.push(renderer.render_template(
        "http/request.zod.ts.j2",
        &RequestZodContext {
            schema_name: format!("{name}Schema"),
            name: name.to_string(),
            params: method.request_fields.clone(),
        },
    )?);
    let _ = module_path;
    Ok(())
}

fn render_response_types(
    out: &mut TsHttpBlocks,
    method: &MethodModel,
    module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<()> {
    let Some(name) = method.response_name.as_ref() else {
        return Ok(());
    };
    let _ = module_path;
    out.types.push(renderer.render_template(
        "http/request.d.ts.j2",
        &RequestContext {
            name: name.to_string(),
            params: method.response_fields.clone(),
            doc: Vec::new(),
        },
    )?);
    out.zod.push(renderer.render_template(
        "http/request.zod.ts.j2",
        &RequestZodContext {
            schema_name: format!("{name}Schema"),
            name: name.to_string(),
            params: method.response_fields.clone(),
        },
    )?);
    Ok(())
}

fn validate_stream_support(op: &HttpOperation) -> IdlcResult<()> {
    match op.stream.kind {
        Some(HttpStreamKind::Server) if op.stream.codec != HttpStreamCodec::Sse => {
            Err(IdlcError::rpc(format!(
                "typescript-http currently supports only SSE for @server_stream methods: '{}'",
                op.name
            )))
        }
        Some(HttpStreamKind::Client) if op.stream.codec != HttpStreamCodec::Ndjson => {
            Err(IdlcError::rpc(format!(
                "typescript-http currently supports only NDJSON for @client_stream methods: '{}'",
                op.name
            )))
        }
        Some(HttpStreamKind::Bidi) => Err(IdlcError::rpc(format!(
            "typescript-http currently does not support @bidi_stream methods: '{}'",
            op.name
        ))),
        _ => Ok(()),
    }
}

fn only_direct_return(op: &HttpOperation) -> bool {
    op.return_type.is_some() && op.response_params.is_empty()
}

fn request_content_type(op: &HttpOperation) -> String {
    match op.stream.kind {
        Some(HttpStreamKind::Client) => "application/x-ndjson".to_string(),
        _ => op.request_content_type.clone(),
    }
}

fn response_content_type(op: &HttpOperation) -> String {
    match (op.stream.kind, op.stream.codec) {
        (Some(HttpStreamKind::Server), HttpStreamCodec::Sse) => "text/event-stream".to_string(),
        _ => op.response_content_type.clone(),
    }
}

fn http_method_name(method: xidl_parser::http_hir::HttpMethod) -> &'static str {
    match method {
        xidl_parser::http_hir::HttpMethod::Get => "GET",
        xidl_parser::http_hir::HttpMethod::Post => "POST",
        xidl_parser::http_hir::HttpMethod::Put => "PUT",
        xidl_parser::http_hir::HttpMethod::Patch => "PATCH",
        xidl_parser::http_hir::HttpMethod::Delete => "DELETE",
        xidl_parser::http_hir::HttpMethod::Head => "HEAD",
        xidl_parser::http_hir::HttpMethod::Options => "OPTIONS",
    }
}

fn client_params(
    op: &HttpOperation,
    module_path: &[String],
    request_name: &Option<String>,
) -> Vec<ClientParamContext> {
    if matches!(op.stream.kind, Some(HttpStreamKind::Client)) {
        return vec![ClientParamContext {
            name: "stream".to_string(),
            ty: client_stream_ty(op, module_path, request_name).unwrap_or_else(|| "never".into()),
        }];
    }
    op.request_params
        .iter()
        .map(|param| ClientParamContext {
            name: ts_ident(&param.name),
            ty: client_param_ty(param, module_path),
        })
        .collect()
}

fn client_param_ty(param: &HttpParam, module_path: &[String]) -> String {
    let base = ts_type_for_type_spec(&param.ty, module_path, TypeRefTarget::Client);
    if param.optional {
        format!("{base} | undefined")
    } else {
        base
    }
}

fn param_decl(param: &HttpParam, module_path: &[String]) -> ParamDeclContext {
    ParamDeclContext {
        prop: crate::generate::typescript::definition::names::ts_prop_name(&param.name),
        ty: ts_type_for_type_spec(&param.ty, module_path, TypeRefTarget::Types),
        schema: zod_schema_for_type_spec(&param.ty, module_path),
        optional: param.optional,
        doc: Vec::new(),
    }
}

fn response_fields(op: &HttpOperation, module_path: &[String]) -> Vec<ParamDeclContext> {
    let mut fields = Vec::new();
    if let Some(ty) = &op.return_type {
        fields.push(ParamDeclContext {
            prop: "return".to_string(),
            ty: ts_type_for_type_spec(ty, module_path, TypeRefTarget::Types),
            schema: zod_schema_for_type_spec(ty, module_path),
            optional: false,
            doc: Vec::new(),
        });
    }
    for param in &op.response_params {
        fields.push(param_decl(param, module_path));
    }
    fields
}

fn value_params(op: &HttpOperation, kind: HttpParamKind) -> Vec<ValueParamContext> {
    op.request_params
        .iter()
        .filter(|param| param.kind == kind)
        .map(|param| ValueParamContext {
            raw_name: param.wire_name.clone(),
            access: ts_ident(&param.name),
            optional: param.optional,
            is_multi: matches!(param.ty, hir::TypeSpec::SequenceType(_)),
        })
        .collect()
}

fn response_value_params(op: &HttpOperation, kind: HttpParamKind) -> Vec<ValueParamContext> {
    op.response_params
        .iter()
        .filter(|param| param.kind == kind)
        .map(|param| ValueParamContext {
            raw_name: param.wire_name.clone(),
            access: crate::generate::typescript::definition::names::ts_prop_name(&param.name),
            optional: param.optional,
            is_multi: matches!(param.ty, hir::TypeSpec::SequenceType(_)),
        })
        .collect()
}

fn direct_body_access(params: &[HttpParam]) -> Option<String> {
    let body = params
        .iter()
        .filter(|param| matches!(param.kind, HttpParamKind::Body))
        .collect::<Vec<_>>();
    if body.len() == 1 && body[0].flatten {
        Some(ts_ident(&body[0].name))
    } else {
        None
    }
}

fn server_stream_ty(op: &HttpOperation, module_path: &[String]) -> Option<String> {
    matches!(op.stream.kind, Some(HttpStreamKind::Server)).then(|| {
        op.return_type
            .as_ref()
            .map(|ty| ts_type_for_type_spec(ty, module_path, TypeRefTarget::Client))
            .unwrap_or_else(|| "unknown".to_string())
    })
}

fn client_stream_ty(
    op: &HttpOperation,
    module_path: &[String],
    request_name: &Option<String>,
) -> Option<String> {
    matches!(op.stream.kind, Some(HttpStreamKind::Client)).then(|| {
        let body = op
            .request_params
            .iter()
            .filter(|param| matches!(param.kind, HttpParamKind::Body))
            .collect::<Vec<_>>();
        let item_ty = if body.len() == 1 && body[0].flatten {
            ts_type_for_type_spec(&body[0].ty, module_path, TypeRefTarget::Client)
        } else if let Some(name) = request_name {
            scoped_name(module_path, name)
        } else {
            "void".to_string()
        };
        format!("AsyncIterable<{item_ty}>")
    })
}

fn security_contexts(op: &HttpOperation) -> Vec<SecurityContext> {
    op.security
        .as_ref()
        .map(|profile| {
            profile
                .requirements
                .iter()
                .map(|value| match value {
                    HttpSecurityRequirement::HttpBasic => SecurityContext {
                        kind: "basic".to_string(),
                        location: None,
                        name: None,
                        realm: op.basic_auth_realm.clone(),
                        scopes: Vec::new(),
                    },
                    HttpSecurityRequirement::HttpBearer => SecurityContext {
                        kind: "bearer".to_string(),
                        location: None,
                        name: None,
                        realm: None,
                        scopes: Vec::new(),
                    },
                    HttpSecurityRequirement::ApiKey { location, name } => SecurityContext {
                        kind: "api_key".to_string(),
                        location: Some(format!("{location:?}").to_ascii_lowercase()),
                        name: Some(name.clone()),
                        realm: None,
                        scopes: Vec::new(),
                    },
                    HttpSecurityRequirement::OAuth2 { scopes } => SecurityContext {
                        kind: "oauth2".to_string(),
                        location: None,
                        name: None,
                        realm: None,
                        scopes: scopes.clone(),
                    },
                })
                .collect()
        })
        .unwrap_or_default()
}

fn response_body_mode(op: &HttpOperation) -> &'static str {
    let response_body_count = op
        .response_params
        .iter()
        .filter(|param| !matches!(param.kind, HttpParamKind::Header | HttpParamKind::Cookie))
        .count();
    match (op.return_type.is_some(), response_body_count) {
        (false, 0) => "none",
        (true, 0) => "return",
        _ => "object",
    }
}
