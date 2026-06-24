use crate::generate::typescript::definition::contexts::{
    ClientParamContext, ParamDeclContext, TsType,
};
use serde::Serialize;

#[derive(Default)]
pub(super) struct TsHttpBlocks {
    pub(super) types: Vec<String>,
    pub(super) zod: Vec<String>,
    pub(super) client: Vec<String>,
}

impl TsHttpBlocks {
    pub(super) fn extend(&mut self, other: Self) {
        self.types.extend(other.types);
        self.zod.extend(other.zod);
        self.client.extend(other.client);
    }

    pub(super) fn is_empty(&self) -> bool {
        self.types.is_empty() && self.zod.is_empty() && self.client.is_empty()
    }
}

#[derive(Clone, Serialize)]
pub(super) struct RequestPayloadEntry {
    pub(super) raw_name: String,
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
    pub(super) raw_name: String,
    pub(super) access: String,
    pub(super) key_name: String,
    pub(super) optional: bool,
    pub(super) is_multi: bool,
}

#[derive(Clone, Serialize)]
pub(super) struct SecurityContext {
    pub(super) kind: String,
    pub(super) location: Option<String>,
    pub(super) name: Option<String>,
    pub(super) realm: Option<String>,
    pub(super) scopes: Vec<String>,
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
    pub(super) is_server_stream: bool,
    pub(super) is_client_stream: bool,
    pub(super) stream_item_ty: Option<TsType>,
    pub(super) stream_item_schema_ref: Option<String>,
    pub(super) client_stream_item_ty: Option<TsType>,
    pub(super) security: Vec<SecurityContext>,
}

pub(super) struct MethodModel {
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
    pub(super) stream_item_ty: Option<TsType>,
    pub(super) stream_item_schema_ref: Option<String>,
    pub(super) client_stream_item_ty: Option<TsType>,
    pub(super) is_server_stream: bool,
    pub(super) is_client_stream: bool,
    pub(super) security: Vec<SecurityContext>,
    pub(super) request_fields: Vec<ParamDeclContext>,
    pub(super) response_fields: Vec<ParamDeclContext>,
}

impl MethodModel {
    pub(super) fn into_client_context(self) -> ClientMethodContext {
        ClientMethodContext {
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
            is_server_stream: self.is_server_stream,
            is_client_stream: self.is_client_stream,
            stream_item_ty: self.stream_item_ty,
            stream_item_schema_ref: self.stream_item_schema_ref,
            client_stream_item_ty: self.client_stream_item_ty,
            security: self.security,
        }
    }
}
