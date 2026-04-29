pub mod annotations;
pub mod attribute;
pub mod context;
pub mod http;
pub mod operation;
pub mod operation_params;
pub mod params;
pub mod types;

#[allow(unused_imports)]
pub use annotations::*;
pub use attribute::*;
#[allow(unused_imports)]
pub use context::*;
#[allow(unused_imports)]
pub use http::*;
pub use operation::*;
#[allow(unused_imports)]
pub use operation_params::*;
#[allow(unused_imports)]
pub use params::*;
#[allow(unused_imports)]
pub use types::*;

use crate::error::{IdlcError, IdlcResult};
use crate::generate::http_hir::HttpOperationSource;
use crate::generate::rust::util::{rust_ident, rust_passthrough_attrs_from_annotations};
use crate::generate::rust_axum::transport::{TransportTracker, TypeRegistry};
use crate::generate::rust_axum::{RustAxumRenderOutput, RustAxumRenderer};
use std::collections::HashMap;
use xidl_parser::hir;

pub fn render_interface_with_path(
    interface: &hir::InterfaceDcl,
    renderer: &RustAxumRenderer,
    module_path: &[String],
    registry: &TypeRegistry,
) -> IdlcResult<RustAxumRenderOutput> {
    match &interface.decl {
        hir::InterfaceDclInner::InterfaceForwardDcl(_) => Ok(RustAxumRenderOutput::default()),
        hir::InterfaceDclInner::InterfaceDef(def) => {
            render_interface_def(def, &interface.annotations, renderer, module_path, registry)
        }
    }
}

pub fn render_interface_def(
    def: &hir::InterfaceDef,
    interface_annotations: &[hir::Annotation],
    renderer: &RustAxumRenderer,
    module_path: &[String],
    registry: &TypeRegistry,
) -> IdlcResult<RustAxumRenderOutput> {
    let mut out = RustAxumRenderOutput::default();
    let mut methods = Vec::new();
    let mut transport = TransportTracker::new(&def.header.ident);
    let http_hir = renderer.http_hir()?;
    let Some(http_interface) = http_hir.find_interface(module_path, &def.header.ident) else {
        return Ok(out);
    };
    if let Some(body) = &def.interface_body {
        for operation in &http_interface.operations {
            match operation.source {
                HttpOperationSource::Method => {
                    let op = body
                        .0
                        .iter()
                        .find_map(|export| match export {
                            hir::Export::OpDcl(op) if op.ident == operation.name => Some(op),
                            _ => None,
                        })
                        .ok_or_else(|| {
                            IdlcError::rpc(format!(
                                "missing source operation '{}' for rust-axum rendering",
                                operation.name
                            ))
                        })?;
                    methods.push(render_op_from_http(
                        op,
                        operation,
                        &def.header.ident,
                        registry,
                        &mut transport,
                    )?);
                }
                HttpOperationSource::AttributeGet
                | HttpOperationSource::AttributeSet
                | HttpOperationSource::AttributeWatch => {
                    let attr = body.0.iter().find_map(|export| match export {
                        hir::Export::AttrDcl(attr)
                            if attr_operation_names(attr).contains(&operation.name) =>
                        {
                            Some(attr)
                        }
                        _ => None,
                    });
                    methods.push(render_attr_operation_from_http(
                        attr,
                        operation,
                        &def.header.ident,
                        registry,
                        &mut transport,
                    )?);
                }
            }
        }
    }
    let transport_modules = transport.render_modules(registry, module_path)?;

    let mut route_bindings = HashMap::new();
    for method in &methods {
        for path in &method.paths {
            let key = format!("{} {}", method.reqwest_method, path);
            if let Some(previous) = route_bindings.insert(key.clone(), method.raw_name.clone()) {
                return Err(IdlcError::rpc(format!(
                    "duplicate HTTP route binding: {key} (methods: {previous}, {})",
                    method.raw_name
                )));
            }
        }
    }
    let ctx = serde_json::json!({
        "metadata": http_hir.document,
        "ident": rust_ident(&def.header.ident),
        "methods": methods,
        "rust_attrs": rust_passthrough_attrs_from_annotations(interface_annotations),
        "inbound_transport_name": transport_modules.inbound_name,
        "outbound_transport_name": transport_modules.outbound_name,
        "inbound_transport": transport_modules.inbound,
        "outbound_transport": transport_modules.outbound,
    });
    let rendered = renderer.render_template("interface.rs.j2", &ctx)?;
    out.source.push(rendered);
    Ok(out)
}
