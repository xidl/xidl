use crate::generate::typescript::definition::contexts::{
    ClientParamContext, ParamDeclContext, TsType, ZodSchema,
};
use crate::generate::typescript::definition::names::scoped_name;
use convert_case::Casing;
use serde::Serialize;

#[derive(Default)]
pub(super) struct TsHttpBlocks {
    pub(super) types: Vec<String>,
    pub(super) zod: Vec<String>,
    pub(super) client: Vec<String>,
    pub(super) server: Vec<String>,
}

impl TsHttpBlocks {
    pub(super) fn extend(&mut self, other: Self) {
        self.types.extend(other.types);
        self.zod.extend(other.zod);
        self.client.extend(other.client);
        self.server.extend(other.server);
    }

    pub(super) fn is_empty(&self) -> bool {
        self.types.is_empty()
            && self.zod.is_empty()
            && self.client.is_empty()
            && self.server.is_empty()
    }
}

#[derive(Clone, Serialize)]
pub(super) struct RequestPayloadEntry {
    pub(super) raw_name: String,
    pub(super) key_name: String,
    pub(super) access: String,
}

#[derive(Clone, Serialize)]
pub(super) struct PathParamContext {
    pub(super) template_name: String,
    pub(super) access: String,
    pub(super) key_name: String,
    pub(super) catch_all: bool,
}

#[derive(Clone, Serialize)]
pub(super) struct ValueParamContext {
    pub(super) item_is_string: bool,
    pub(super) raw_name: String,
    pub(super) access: String,
    pub(super) key_name: String,
    pub(super) optional: bool,
    pub(super) is_multi: bool,
}

impl ValueParamContext {
    pub(super) fn item_is_string(ty: &xidl_parser::hir::TypeSpec) -> bool {
        use xidl_parser::hir::TypeSpec;
        match ty {
            TypeSpec::SequenceType(sequence) => Self::item_is_string(&sequence.ty),
            TypeSpec::StringType(_)
            | TypeSpec::WideStringType(_)
            | TypeSpec::CharType
            | TypeSpec::WideCharType => true,
            _ => false,
        }
    }
}

#[derive(Clone, Serialize)]
pub(super) struct SecurityContext {
    pub(super) kind: String,
    pub(super) location: Option<String>,
    pub(super) name: Option<String>,
    pub(super) realm: Option<String>,
}

/// One representation of an `@http` union response (typescript-rest).
#[derive(Clone, Serialize)]
pub(super) struct TsRepresentationContext {
    pub(super) kind: String,
    pub(super) content_type: String,
    pub(super) value_ty: TsType,
    pub(super) schema: ZodSchema,
    pub(super) is_byte: bool,
}

/// One `raises(...)` entry projected for the typescript-rest generator.
#[derive(Clone, Serialize)]
pub(super) struct TsRaisesContext {
    pub(super) ident: String,
    pub(super) status: u16,
    pub(super) has_body: bool,
    /// `@header` members: written to / read from the error response.
    pub(super) headers: Vec<TsExceptionMemberContext>,
    pub(super) cookies: Vec<TsExceptionMemberContext>,
    /// Body members: JSON fields of the error response.
    pub(super) body: Vec<TsExceptionFieldContext>,
}

impl TsRaisesContext {
    pub(super) fn error_ident(
        exception: &xidl_parser::rest_hir::HttpException,
        exceptions: &[xidl_parser::rest_hir::HttpException],
    ) -> String {
        if exceptions
            .iter()
            .filter(|other| other.ident == exception.ident)
            .count()
            == 1
        {
            exception.ident.clone()
        } else {
            exception
                .module_path
                .iter()
                .chain(std::iter::once(&exception.ident))
                .cloned()
                .collect::<Vec<_>>()
                .join("$")
        }
    }
}

#[derive(Clone, Serialize)]
pub(super) struct TsExceptionMemberContext {
    pub(super) item_is_string: bool,
    pub(super) field: String,
    pub(super) wire_name: String,
    pub(super) ty: TsType,
    pub(super) is_multi: bool,
    pub(super) optional: bool,
}

#[derive(Clone, Serialize)]
pub(super) struct TsExceptionFieldContext {
    pub(super) schema: ZodSchema,
    pub(super) optional: bool,
    pub(super) field: String,
    pub(super) wire_name: String,
    pub(super) ty: TsType,
}

#[derive(Serialize)]
pub(super) struct ClientClassContext {
    pub(super) client_name: String,
    pub(super) methods: Vec<ClientMethodContext>,
}

#[derive(Serialize)]
pub(super) struct ClientMethodContext {
    pub(super) name: String,
    pub(super) params: Vec<ClientParamContext>,
    pub(super) return_ty: TsType,
    pub(super) request_schema_ref: Option<String>,
    pub(super) body_schema_ref: Option<String>,
    pub(super) request_payload: Vec<RequestPayloadEntry>,
    pub(super) path: String,
    pub(super) http_method: String,
    pub(super) request_content_type: String,
    pub(super) response_content_type: String,
    pub(super) path_params: Vec<PathParamContext>,
    pub(super) query_params: Vec<ValueParamContext>,
    pub(super) header_params: Vec<ValueParamContext>,
    pub(super) cookie_params: Vec<ValueParamContext>,
    pub(super) response_header_params: Vec<ValueParamContext>,
    pub(super) response_cookie_params: Vec<ValueParamContext>,
    pub(super) body_entries: Vec<RequestPayloadEntry>,
    pub(super) body_single: Option<String>,
    pub(super) response_schema_ref: Option<String>,
    pub(super) response_body_mode: String,
    pub(super) response_body_entries: Vec<RequestPayloadEntry>,
    pub(super) is_server_stream: bool,
    pub(super) is_client_stream: bool,
    pub(super) is_websocket: bool,
    pub(super) websocket_subprotocol: Option<String>,
    pub(super) stream_in_ty: String,
    pub(super) stream_out_ty: String,
    pub(super) stream_item_ty: Option<TsType>,
    pub(super) stream_item_schema_ref: Option<String>,
    pub(super) security: Vec<SecurityContext>,
    pub(super) raises: Vec<TsRaisesContext>,
    pub(super) raise_helper: String,
    pub(super) representations: Vec<TsRepresentationContext>,
    pub(super) union_wrapper: Option<TsType>,
    pub(super) union_accept: String,
}

#[derive(Serialize)]
pub(super) struct ServerClassContext {
    pub(super) service_name: String,
    pub(super) methods: Vec<ServerMethodContext>,
}

#[derive(Serialize)]
pub(super) struct ServerMethodContext {
    pub(super) response_status: String,
    pub(super) name: String,
    pub(super) params: Vec<ClientParamContext>,
    pub(super) response_ty: TsType,
    pub(super) request_schema_ref: Option<String>,
    pub(super) body_schema_ref: Option<String>,
    pub(super) response_schema_ref: Option<String>,
    pub(super) path: String,
    pub(super) paths: Vec<String>,
    pub(super) http_method: String,
    pub(super) request_content_type: String,
    pub(super) response_content_type: String,
    pub(super) path_params: Vec<PathParamContext>,
    pub(super) query_params: Vec<ValueParamContext>,
    pub(super) header_params: Vec<ValueParamContext>,
    pub(super) cookie_params: Vec<ValueParamContext>,
    pub(super) response_header_params: Vec<ValueParamContext>,
    pub(super) response_cookie_params: Vec<ValueParamContext>,
    pub(super) body_entries: Vec<RequestPayloadEntry>,
    pub(super) body_single_key: Option<String>,
    pub(super) response_body_entries: Vec<RequestPayloadEntry>,
    pub(super) response_body_mode: String,
    pub(super) is_server_stream: bool,
    pub(super) is_client_stream: bool,
    pub(super) is_websocket: bool,
    pub(super) websocket_subprotocol: Option<String>,
    pub(super) stream_in_ty: String,
    pub(super) stream_out_ty: String,
    pub(super) stream_item_ty: Option<TsType>,
    pub(super) stream_item_schema_ref: Option<String>,
    pub(super) security: Vec<SecurityContext>,
    pub(super) raises: Vec<TsRaisesContext>,
    pub(super) representations: Vec<TsRepresentationContext>,
    pub(super) union_wrapper: Option<TsType>,
}

#[derive(Clone)]
pub(super) struct MethodModel {
    pub(super) response_status: String,
    pub(super) name: String,
    pub(super) params: Vec<ClientParamContext>,
    pub(super) request_name: Option<String>,
    pub(super) request_schema_ref: Option<String>,
    pub(super) body_schema_ref: Option<String>,
    pub(super) request_payload: Vec<RequestPayloadEntry>,
    pub(super) response_name: Option<String>,
    pub(super) response_schema_ref: Option<String>,
    pub(super) request_content_type: String,
    pub(super) response_content_type: String,
    pub(super) path: String,
    pub(super) paths: Vec<String>,
    pub(super) http_method: String,
    pub(super) path_params: Vec<PathParamContext>,
    pub(super) query_params: Vec<ValueParamContext>,
    pub(super) header_params: Vec<ValueParamContext>,
    pub(super) cookie_params: Vec<ValueParamContext>,
    pub(super) response_header_params: Vec<ValueParamContext>,
    pub(super) response_cookie_params: Vec<ValueParamContext>,
    pub(super) body_entries: Vec<RequestPayloadEntry>,
    pub(super) body_single: Option<String>,
    pub(super) return_ty: TsType,
    pub(super) response_body_mode: String,
    pub(super) response_body_entries: Vec<RequestPayloadEntry>,
    pub(super) stream_item_ty: Option<TsType>,
    pub(super) stream_item_schema_ref: Option<String>,
    pub(super) is_server_stream: bool,
    pub(super) is_client_stream: bool,
    pub(super) is_websocket: bool,
    pub(super) websocket_subprotocol: Option<String>,
    pub(super) stream_in_ty: String,
    pub(super) stream_out_ty: String,
    pub(super) security: Vec<SecurityContext>,
    pub(super) request_fields: Vec<ParamDeclContext>,
    pub(super) response_fields: Vec<ParamDeclContext>,
    pub(super) raises: Vec<TsRaisesContext>,
    /// `@http` union representations; non-empty switches the response flow.
    pub(super) representations: Vec<TsRepresentationContext>,
    /// Response wrapper for `@http` unions:
    /// `{ kind; value; <out params> } | ...`.
    pub(super) union_wrapper: Option<TsType>,
}

impl MethodModel {
    pub(super) fn into_client_context(self) -> ClientMethodContext {
        ClientMethodContext {
            raise_helper: format!(
                "throw{}Error",
                self.name.to_case(convert_case::Case::Pascal)
            ),
            name: self.name,
            params: self.params,
            return_ty: self.return_ty,
            request_schema_ref: self.request_schema_ref,
            body_schema_ref: self.body_schema_ref,
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
            response_body_entries: self.response_body_entries,
            is_server_stream: self.is_server_stream,
            is_client_stream: self.is_client_stream,
            is_websocket: self.is_websocket,
            websocket_subprotocol: self.websocket_subprotocol.clone(),
            stream_in_ty: self.stream_in_ty.clone(),
            stream_out_ty: self.stream_out_ty.clone(),
            stream_item_ty: self.stream_item_ty,
            stream_item_schema_ref: self.stream_item_schema_ref,
            security: self.security,
            raises: self.raises.clone(),
            representations: self.representations.clone(),
            union_wrapper: self.union_wrapper.clone(),
            union_accept: self
                .representations
                .iter()
                .map(|repr| repr.content_type.clone())
                .collect::<Vec<_>>()
                .join(", "),
        }
    }

    pub(super) fn into_server_context(self, module_path: &[String]) -> ServerMethodContext {
        let response_ty = self
            .response_name
            .as_ref()
            .map(|name| {
                TsType::ScopedName(format!("ifaceTypes.{}", scoped_name(module_path, name)))
            })
            .unwrap_or(self.return_ty);
        ServerMethodContext {
            response_status: self.response_status,
            name: self.name,
            params: self.params,
            response_ty,
            request_schema_ref: self.request_schema_ref,
            body_schema_ref: self.body_schema_ref,
            response_schema_ref: self.response_schema_ref,
            path: self.path,
            paths: self.paths,
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
            body_single_key: self.body_single,
            response_body_entries: self.response_body_entries,
            response_body_mode: self.response_body_mode,
            is_server_stream: self.is_server_stream,
            is_client_stream: self.is_client_stream,
            is_websocket: self.is_websocket,
            websocket_subprotocol: self.websocket_subprotocol.clone(),
            stream_in_ty: self.stream_in_ty.clone(),
            stream_out_ty: self.stream_out_ty.clone(),
            stream_item_ty: self.stream_item_ty,
            stream_item_schema_ref: self.stream_item_schema_ref,
            security: self.security,
            raises: self.raises,
            representations: self.representations,
            union_wrapper: self.union_wrapper,
        }
    }
}
