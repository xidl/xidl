mod interface_annotations;
mod interface_attr;
mod interface_attr_support;
mod interface_model;
mod interface_names;
mod interface_ops;
mod interface_ops_support;
mod interface_render;
mod interface_types;

use crate::error::IdlcResult;
use crate::generate::rust_jsonrpc::{JsonRpcRenderOutput, JsonRpcRenderer};
use xidl_parser::hir;

pub fn render_interface_with_path(
    interface: &hir::InterfaceDcl,
    renderer: &JsonRpcRenderer,
    module_path: &[String],
) -> IdlcResult<JsonRpcRenderOutput> {
    match &interface.decl {
        hir::InterfaceDclInner::InterfaceForwardDcl(_) => Ok(JsonRpcRenderOutput::default()),
        hir::InterfaceDclInner::InterfaceDef(def) => interface_render::render_interface_def(
            def,
            &interface.annotations,
            renderer,
            module_path,
        ),
    }
}
