use serde::Serialize;

#[derive(Serialize)]
pub(super) struct ParamField {
    pub(super) name: String,
    pub(super) ty: String,
}

#[derive(Serialize)]
pub(super) struct OutputField {
    pub(super) name: String,
    pub(super) json_name: String,
    pub(super) ty: String,
}

#[derive(Serialize)]
pub(super) struct MethodContext {
    pub(super) kind: String,
    pub(super) stream_mode: String,
    pub(super) name: String,
    pub(super) rust_attrs: Vec<String>,
    pub(super) params: Vec<String>,
    pub(super) params_fields: Vec<ParamField>,
    pub(super) params_struct: String,
    pub(super) ret: String,
    pub(super) rpc_name: String,
    pub(super) args: Vec<String>,
    pub(super) response_kind: String,
    pub(super) response_struct: String,
    pub(super) response_fields: Vec<OutputField>,
    pub(super) response_single_field: String,
    pub(super) stream_item_ty: String,
}

#[derive(Serialize)]
pub(super) struct WatchMethodContext {
    pub(super) getter_name: String,
    pub(super) item_ty: String,
    pub(super) stream_rpc_name: String,
}

pub(super) struct RenderedAttr {
    pub(super) methods: Vec<MethodContext>,
    pub(super) watch_methods: Vec<WatchMethodContext>,
}

pub(super) struct AttrNames {
    pub(super) raw_attr: String,
    pub(super) raw_getter: String,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum StreamKind {
    Server,
    Client,
    Bidi,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub(super) enum ParamMode {
    In,
    Out,
    InOut,
}
