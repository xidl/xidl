mod interface_annotations;
mod interface_attr;
mod interface_http;
mod interface_method;
mod interface_method_params;
mod interface_method_support;
mod interface_model;
mod interface_render;
mod interface_types;

use crate::error::IdlcResult;
use crate::generate::rust_axum::transport::TypeRegistry;
use crate::generate::rust_axum::{RustAxumRenderOutput, RustAxumRenderer};
use xidl_parser::hir;

pub(crate) use interface_model::{
    ApiKeyContext, DeprecatedContext, HttpMethod, MethodContext, ParamContext, ParamSource,
    RenderEnv,
};

pub fn render_interface_with_path(
    interface: &hir::InterfaceDcl,
    renderer: &RustAxumRenderer,
    module_path: &[String],
    registry: &TypeRegistry,
) -> IdlcResult<RustAxumRenderOutput> {
    match &interface.decl {
        hir::InterfaceDclInner::InterfaceForwardDcl(_) => Ok(RustAxumRenderOutput::default()),
        hir::InterfaceDclInner::InterfaceDef(def) => interface_render::render_interface_def(
            def,
            &interface.annotations,
            RenderEnv::new(renderer, module_path, registry),
        ),
    }
}
