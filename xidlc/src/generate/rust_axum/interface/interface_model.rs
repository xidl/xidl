use crate::generate::rust_axum::RustAxumRenderer;
use crate::generate::rust_axum::transport::TypeRegistry;
use serde::Serialize;

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

#[derive(Serialize)]
pub(crate) struct MethodContext {
    pub(crate) name: String,
    pub(crate) raw_name: String,
    pub(crate) rust_attrs: Vec<String>,
    pub(crate) deprecated: bool,
    pub(crate) deprecated_since: Option<String>,
    pub(crate) deprecated_after: Option<String>,
    pub(crate) deprecated_note: Option<String>,
    pub(crate) params: Vec<String>,
    pub(crate) param_names: Vec<String>,
    pub(crate) server_params: Vec<String>,
    pub(crate) server_param_names: Vec<String>,
    pub(crate) ret: String,
    pub(crate) response_ty: String,
    pub(crate) request_body_flatten: bool,
    pub(crate) http_method: String,
    pub(crate) http_method_fn: String,
    pub(crate) reqwest_method: String,
    pub(crate) path: String,
    pub(crate) paths: Vec<String>,
    pub(crate) struct_prefix: String,
    pub(crate) path_params: Vec<ParamContext>,
    pub(crate) query_params: Vec<ParamContext>,
    pub(crate) header_params: Vec<ParamContext>,
    pub(crate) cookie_params: Vec<ParamContext>,
    pub(crate) body_params: Vec<ParamContext>,
    pub(crate) request_ty: String,
    pub(crate) request_payload_ty: String,
    pub(crate) request_struct: Option<String>,
    pub(crate) auth_wrapper_struct: Option<String>,
    pub(crate) auth_in_request_struct: bool,
    pub(crate) has_basic_auth: bool,
    pub(crate) has_bearer_auth: bool,
    pub(crate) api_key_requirements: Vec<ApiKeyContext>,
    pub(crate) auth_source_interface: bool,
    pub(crate) auth_source_method: bool,
    pub(crate) auth_param: Option<String>,
    pub(crate) auth_param_ty: String,
    pub(crate) auth_ty: String,
    pub(crate) basic_auth_realm: String,
    pub(crate) request_params: Vec<ParamContext>,
    pub(crate) response_struct: Option<String>,
    pub(crate) response_params: Vec<ParamContext>,
    pub(crate) response_body_params: Vec<ParamContext>,
    pub(crate) response_header_params: Vec<ParamContext>,
    pub(crate) response_cookie_params: Vec<ParamContext>,
    pub(crate) response_include_return: bool,
    pub(crate) response_is_empty: bool,
    pub(crate) return_is_unit: bool,
    pub(crate) is_server_stream: bool,
    pub(crate) is_client_stream: bool,
    pub(crate) is_bidi_stream: bool,
    pub(crate) request_item_ty: String,
    pub(crate) ret_in_ty: String,
    pub(crate) ret_out_ty: String,
    pub(crate) ret_in_expr: String,
    pub(crate) ret_out_expr: String,
    pub(crate) request_content_type: String,
    pub(crate) response_content_type: String,
}

#[derive(Serialize, Clone)]
pub(crate) struct ParamContext {
    pub(crate) name: String,
    pub(crate) raw_name: String,
    pub(crate) wire_name: String,
    pub(crate) path_template_name: String,
    pub(crate) ty: String,
    pub(crate) in_ty: String,
    pub(crate) out_ty: String,
    pub(crate) source: String,
    pub(crate) serde_rename: Option<String>,
    pub(crate) header_is_multi: bool,
    pub(crate) header_item_ty: String,
    pub(crate) header_item_is_string: bool,
    pub(crate) header_item_is_primitive: bool,
    pub(crate) cookie_is_multi: bool,
    pub(crate) cookie_item_ty: String,
    pub(crate) cookie_item_is_string: bool,
    pub(crate) cookie_item_is_primitive: bool,
    pub(crate) optional: bool,
    pub(crate) inner_ty: String,
    pub(crate) flatten: bool,
    pub(crate) in_expr: String,
    pub(crate) out_expr: String,
    pub(crate) field_in_expr: String,
    pub(crate) field_out_expr: String,
}

#[derive(Serialize, Clone)]
pub(crate) struct ApiKeyContext {
    pub(crate) location: String,
    pub(crate) name: String,
}

pub(crate) struct DeprecatedContext {
    pub(crate) deprecated: bool,
    pub(crate) since: Option<String>,
    pub(crate) after: Option<String>,
    pub(crate) note: Option<String>,
}

#[derive(Clone, Copy)]
pub(crate) struct RenderEnv<'a> {
    pub(crate) renderer: &'a RustAxumRenderer,
    pub(crate) module_path: &'a [String],
    pub(crate) registry: &'a TypeRegistry,
}

impl<'a> RenderEnv<'a> {
    pub(crate) fn new(
        renderer: &'a RustAxumRenderer,
        module_path: &'a [String],
        registry: &'a TypeRegistry,
    ) -> Self {
        Self {
            renderer,
            module_path,
            registry,
        }
    }
}
