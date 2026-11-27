use crate::generate::rust_axum::interface::interface_model::MethodContext;
use serde::Serialize;

/// Precomputed template branches for one HTTP operation.
#[derive(Clone, Default, Serialize)]
pub(crate) struct MethodFlags {
    /// True for server, client, or bidirectional streaming.
    pub(crate) is_streaming: bool,
    /// True for any stream or raw upgrade handler shape.
    pub(crate) is_streaming_or_upgrade: bool,
    /// True when the operation declares a typed error channel.
    pub(crate) has_raises: bool,
    /// A response struct exists and carries a wire body.
    pub(crate) response_has_body: bool,
    /// The response carries headers or cookies.
    pub(crate) has_response_metadata: bool,
    /// The response is a content-negotiated union.
    pub(crate) has_representations: bool,
    /// The request body is transferred as raw bytes.
    pub(crate) is_binary_body: bool,
    /// The operation requires client-side auth injection.
    pub(crate) has_auth_requirements: bool,
    /// Trait method return category.
    pub(crate) trait_return_kind: &'static str,
    /// Client method return category.
    pub(crate) client_return_kind: &'static str,
    /// Server auth extraction category.
    pub(crate) auth_kind: &'static str,
    /// Server response construction category.
    pub(crate) response_render_kind: &'static str,
    /// Client request setup category.
    pub(crate) client_request_kind: &'static str,
    /// Client response handling category.
    pub(crate) client_response_kind: &'static str,
    /// Handler receives path parameters as a dedicated extractor.
    pub(crate) handler_uses_path: bool,
    /// Handler receives query parameters as a dedicated extractor.
    pub(crate) handler_uses_query: bool,
    /// Handler receives a generated request struct extractor.
    pub(crate) handler_uses_request: bool,
}

impl MethodFlags {
    pub(crate) fn from_method(method: &MethodContext) -> Self {
        let is_streaming =
            method.is_server_stream || method.is_client_stream || method.is_bidi_stream;
        let is_streaming_or_upgrade = is_streaming || method.is_upgrade;
        let is_binary_body = method.request_content_type.starts_with("application/")
            && method.request_content_type.ends_with("/octet-stream");
        let response_body_count =
            method.response_body_params.len() + usize::from(!method.return_is_unit);
        Self {
            is_streaming,
            is_streaming_or_upgrade,
            has_raises: !method.raises.is_empty(),
            response_has_body: method.response_struct.is_some() && response_body_count > 0,
            has_response_metadata: !method.response_header_params.is_empty()
                || !method.response_cookie_params.is_empty(),
            has_representations: !method.representations.is_empty(),
            is_binary_body,
            has_auth_requirements: (method.auth_source_interface || method.auth_source_method)
                && (method.has_basic_auth
                    || method.has_bearer_auth
                    || !method.api_key_requirements.is_empty()),
            trait_return_kind: Self::trait_return_kind(method),
            client_return_kind: Self::client_return_kind(method),
            auth_kind: Self::auth_kind(method),
            response_render_kind: Self::response_render_kind(method),
            client_request_kind: Self::client_request_kind(method, is_binary_body),
            client_response_kind: Self::client_response_kind(method),
            handler_uses_path: !method.path_params.is_empty() && is_streaming_or_upgrade,
            handler_uses_query: !method.query_params.is_empty() && is_streaming_or_upgrade,
            handler_uses_request: method.request_struct.is_some() && !is_streaming_or_upgrade,
        }
    }

    fn trait_return_kind(method: &MethodContext) -> &'static str {
        if method.is_server_stream {
            if method.is_byte_stream {
                "byte_stream"
            } else {
                "sse_stream"
            }
        } else if method.is_bidi_stream {
            "unit"
        } else if method.is_upgrade {
            "upgrade"
        } else {
            "response"
        }
    }

    fn client_return_kind(method: &MethodContext) -> &'static str {
        if method.is_client_stream {
            if method.is_byte_stream {
                "byte_writer"
            } else {
                "writer"
            }
        } else if method.is_bidi_stream {
            "reader_writer"
        } else if method.is_server_stream {
            if method.is_byte_stream {
                "byte_reader"
            } else {
                "reader"
            }
        } else if method.is_upgrade {
            "upgraded"
        } else {
            "plain"
        }
    }

    fn auth_kind(method: &MethodContext) -> &'static str {
        if method.auth_ty.ends_with("BasicAuth") {
            "basic"
        } else if method.auth_ty.ends_with("BearerAuth") {
            "bearer"
        } else if method.auth_ty.ends_with("ApiKeyAuth") {
            "api_key"
        } else if method.auth_ty.ends_with("ClientAuth") {
            "client"
        } else {
            "none"
        }
    }

    fn response_render_kind(method: &MethodContext) -> &'static str {
        if method.is_server_stream {
            if method.is_byte_stream {
                "byte_stream"
            } else {
                "sse_stream"
            }
        } else if method.is_upgrade {
            "upgrade"
        } else if !method.representations.is_empty() {
            "representations"
        } else if method.response_is_empty {
            "empty"
        } else {
            "body"
        }
    }

    fn client_request_kind(method: &MethodContext, is_binary_body: bool) -> &'static str {
        if method.is_bidi_stream {
            "bidi"
        } else if method.is_client_stream {
            "client_stream"
        } else if !method.body_params.is_empty() && is_binary_body {
            "binary_body"
        } else if !method.body_params.is_empty() {
            "body"
        } else {
            "none"
        }
    }

    fn client_response_kind(method: &MethodContext) -> &'static str {
        if method.is_server_stream {
            "server_stream"
        } else if method.is_upgrade {
            "upgrade"
        } else if !method.is_client_stream && !method.is_bidi_stream {
            "unary"
        } else {
            "none"
        }
    }
}

impl MethodContext {
    pub(crate) fn with_computed_flags(mut self) -> Self {
        self.flags = MethodFlags::from_method(&self);
        self
    }
}
