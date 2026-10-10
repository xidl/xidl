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
    pub(crate) response_body_flatten: bool,
    pub(crate) http_method: String,
    pub(crate) http_method_fn: String,
    pub(crate) reqwest_method: String,
    pub(crate) path: String,
    pub(crate) paths: Vec<String>,
    pub(crate) cors_layer: Option<String>,
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
    pub(crate) is_byte_stream: bool,
    pub(crate) request_item_ty: String,
    pub(crate) ret_in_ty: String,
    pub(crate) ret_out_ty: String,
    pub(crate) ret_in_expr: String,
    pub(crate) ret_out_expr: String,
    pub(crate) request_content_type: String,
    pub(crate) response_content_type: String,
    pub(crate) response_status: String,
    pub(crate) is_upgrade: bool,
    pub(crate) upgrade_protocol: Option<String>,
    pub(crate) is_upgrade_websocket: bool,
    pub(crate) websocket_subprotocol: Option<String>,
    pub(crate) websocket_heartbeat_ms: Option<u64>,
    pub(crate) websocket_max_message_bytes: Option<u64>,
    pub(crate) handshake_params: Vec<String>,
    pub(crate) handshake_param_names: Vec<String>,
    /// Exceptions from `raises(...)`: typed error variants for this operation.
    pub(crate) raises: Vec<RaisesContext>,
    /// The operation's error type: the per-operation enum when `raises` is
    /// non-empty, otherwise the runtime [`xidl_rust_axum::Error`].
    pub(crate) error_ty: Option<String>,
}

/// One `raises(...)` entry projected for the rust-axum generator.
#[derive(Serialize, Clone)]
pub(crate) struct RaisesContext {
    /// Enum variant identifier (Pascal-cased exception name).
    pub(crate) variant: String,
    /// Exception struct path as written in the IDL (resolves in-module).
    pub(crate) ty: String,
    pub(crate) status: u16,
    pub(crate) headers: Vec<ExceptionMemberContext>,
    pub(crate) cookies: Vec<ExceptionMemberContext>,
    /// Whether the exception declares body members (non-`@header`/`@cookie`).
    pub(crate) has_body: bool,
}

/// A `@header`/`@cookie` member of an exception, for wire read/write.
#[derive(Serialize, Clone)]
pub(crate) struct ExceptionMemberContext {
    pub(crate) field: String,
    pub(crate) wire_name: String,
    pub(crate) ty: String,
    pub(crate) is_multi: bool,
    pub(crate) item_ty: String,
    pub(crate) item_is_string: bool,
    pub(crate) item_is_primitive: bool,
    /// `@optional`: the field is `Option<_>`; absent means no header.
    pub(crate) optional: bool,
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
    pub(crate) fn relative_type_path(&self, target: &[String], ident: &str) -> String {
        let common = self
            .module_path
            .iter()
            .zip(target)
            .take_while(|(a, b)| a == b)
            .count();
        std::iter::repeat_n("super".to_string(), self.module_path.len() - common)
            .chain(
                target[common..]
                    .iter()
                    .map(|part| crate::generate::rust::util::rust_ident(part)),
            )
            .chain(std::iter::once(crate::generate::rust::util::rust_ident(
                ident,
            )))
            .collect::<Vec<_>>()
            .join("::")
    }

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
