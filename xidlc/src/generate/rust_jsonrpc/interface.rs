#[path = "interface_annotations.rs"]
mod interface_annotations;
#[path = "interface_attr.rs"]
mod interface_attr;
#[path = "interface_attr_support.rs"]
mod interface_attr_support;
#[path = "interface_model.rs"]
mod interface_model;
#[path = "interface_names.rs"]
mod interface_names;
#[path = "interface_ops.rs"]
mod interface_ops;
#[path = "interface_ops_support.rs"]
mod interface_ops_support;
#[path = "interface_render.rs"]
mod interface_render;
#[path = "interface_types.rs"]
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
