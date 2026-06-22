use crate::error::IdlcResult;
use crate::generate::rust::util::rust_passthrough_attrs_from_annotations;
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use crate::generate::utils::doc_lines_from_annotations;
use serde_json::json;
use xidl_parser::hir;

mod interface_dcl_support;
use interface_dcl_support::InterfaceMethodRenderer;

impl RustRender for hir::InterfaceDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        match &self.decl {
            hir::InterfaceDclInner::InterfaceForwardDcl(forward) => forward.render(renderer),
            hir::InterfaceDclInner::InterfaceDef(def) => render_interface_def_with_doc(
                def,
                doc_lines_from_annotations(&self.annotations),
                renderer,
                rust_passthrough_attrs_from_annotations(&self.annotations),
            ),
        }
    }
}

impl RustRender for hir::InterfaceForwardDcl {
    fn render(&self, _renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        Ok(RustRenderOutput::empty())
    }
}

impl RustRender for hir::InterfaceDef {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        render_interface_def_with_doc(self, Vec::new(), renderer, Vec::new())
    }
}

fn render_interface_def_with_doc(
    def: &hir::InterfaceDef,
    doc: Vec<String>,
    renderer: &RustRenderer,
    rust_attrs: Vec<String>,
) -> IdlcResult<RustRenderOutput> {
    let mut out = RustRenderOutput::default();
    let method_renderer = InterfaceMethodRenderer::new();
    let mut methods = Vec::new();

    if let Some(body) = &def.interface_body {
        for export in &body.0 {
            match export {
                hir::Export::OpDcl(op) => {
                    methods.push(method_renderer.render_op(op));
                }
                hir::Export::AttrDcl(attr) => {
                    methods.extend(method_renderer.render_attr(attr));
                }
                hir::Export::TypeDcl(type_dcl) => {
                    out.extend(type_dcl.render(renderer)?);
                }
                hir::Export::ConstDcl(const_dcl) => {
                    out.extend(const_dcl.render(renderer)?);
                }
                hir::Export::ExceptDcl(except) => {
                    out.extend(except.render(renderer)?);
                }
            }
        }
    }

    let ctx = json!({
        "ident": crate::generate::rust::util::rust_ident(&def.header.ident),
        "methods": methods,
        "doc": doc,
        "rust_attrs": rust_attrs,
    });
    out.extend(renderer.render_source_template("interface.rs.j2", &ctx)?);
    Ok(out)
}
