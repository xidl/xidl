mod interface_model;
mod interface_names;
mod interface_ops_support;
mod interface_render;
mod interface_types;

use crate::error::IdlcResult;
use crate::generate::rust_jsonrpc::{JsonRpcRenderOutput, JsonRpcRenderer};
use xidl_parser::jsonrpc_hir::JsonRpcInterface;

pub fn render_interface_with_path(
    interface: &JsonRpcInterface,
    renderer: &JsonRpcRenderer,
) -> IdlcResult<JsonRpcRenderOutput> {
    interface_render::render_interface_def(interface, renderer)
}
