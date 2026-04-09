use crate::error::IdlcResult;
use crate::generate::cpp::{CppRender, CppRenderOutput, CppRenderer};
use serde_json::json;
use xidl_parser::hir;

#[path = "interface_dcl_support.rs"]
mod interface_dcl_support;
use interface_dcl_support::{parent_names, render_attr, render_op};

impl CppRender for hir::InterfaceDcl {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        match &self.decl {
            hir::InterfaceDclInner::InterfaceForwardDcl(forward) => forward.render(renderer),
            hir::InterfaceDclInner::InterfaceDef(def) => def.render(renderer),
        }
    }
}

impl CppRender for hir::InterfaceForwardDcl {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        let ctx = json!({ "kind": "class", "ident": &self.ident });
        let rendered = renderer.render_template("forward.h.j2", &ctx)?;
        Ok(CppRenderOutput::default().push_header(rendered))
    }
}

impl CppRender for hir::InterfaceDef {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        let mut out = CppRenderOutput::default();

        let parents = parent_names(self);

        let mut methods = Vec::new();
        let mut nested_lines = Vec::new();
        if let Some(body) = &self.interface_body {
            for export in &body.0 {
                match export {
                    hir::Export::OpDcl(op) => {
                        methods.push(render_op(op));
                    }
                    hir::Export::AttrDcl(attr) => {
                        methods.extend(render_attr(attr));
                    }
                    hir::Export::TypeDcl(type_dcl) => {
                        let rendered = type_dcl.render(renderer)?;
                        for header in rendered.header {
                            nested_lines.extend(header.lines().map(|line| line.to_string()));
                        }
                        out.source.extend(rendered.source);
                    }
                    hir::Export::ConstDcl(const_dcl) => {
                        let rendered = const_dcl.render(renderer)?;
                        for header in rendered.header {
                            nested_lines.extend(header.lines().map(|line| line.to_string()));
                        }
                        out.source.extend(rendered.source);
                    }
                    hir::Export::ExceptDcl(except) => {
                        let rendered = except.render(renderer)?;
                        for header in rendered.header {
                            nested_lines.extend(header.lines().map(|line| line.to_string()));
                        }
                    }
                }
            }
        }

        let ctx = json!({
            "ident": &self.header.ident,
            "parents": parents,
            "methods": methods,
            "nested_lines": nested_lines,
        });
        let rendered = renderer.render_template("interface.h.j2", &ctx)?;
        out.header.push(rendered);
        Ok(out)
    }
}
