use crate::error::IdlcResult;
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use std::collections::HashMap;
use xidl_parser::hir;

impl RustRender for hir::ModuleDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let defs = self.definition.iter().collect::<Vec<_>>();
        let body = render_module_body(&defs, renderer)?;
        let rendered = renderer.render_template(
            "module.rs.j2",
            &serde_json::json!({
                "ident": crate::generate::rust::util::rust_ident(&self.ident),
                "body": body,
            }),
        )?;
        Ok(RustRenderOutput::default().push_source(rendered))
    }
}

impl RustRender for hir::Definition {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        match self {
            hir::Definition::ModuleDcl(module) => module.render(renderer),
            hir::Definition::ConstrTypeDcl(constr) => constr.render(renderer),
            hir::Definition::TypeDcl(type_dcl) => type_dcl.render(renderer),
            hir::Definition::ConstDcl(const_dcl) => const_dcl.render(renderer),
            hir::Definition::ExceptDcl(except_dcl) => except_dcl.render(renderer),
            hir::Definition::InterfaceDcl(interface) => interface.render(renderer),
        }
    }
}

pub(crate) fn indent_lines(value: &str, prefix: &str) -> String {
    value
        .lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn render_module_body(
    defs: &[&hir::Definition],
    renderer: &RustRenderer,
) -> IdlcResult<String> {
    let mut out = Vec::new();
    let mut module_order = Vec::new();
    let mut module_map: HashMap<String, Vec<&hir::ModuleDcl>> = HashMap::new();

    for def in defs {
        match def {
            hir::Definition::ModuleDcl(module) => {
                let entry = module_map.entry(module.ident.clone()).or_insert_with(|| {
                    module_order.push(module.ident.clone());
                    Vec::new()
                });
                entry.push(module);
            }
            other => {
                let rendered = other.render(renderer)?;
                out.extend(rendered.source);
            }
        }
    }

    for name in module_order {
        let modules = module_map.remove(&name).unwrap_or_default();
        let mut inner_defs = Vec::new();
        for module in modules {
            for def in &module.definition {
                inner_defs.push(def);
            }
        }
        let body = render_module_body(&inner_defs, renderer)?;
        let rendered = renderer.render_template(
            "module.rs.j2",
            &serde_json::json!({
                "ident": crate::generate::rust::util::rust_ident(&name),
                "body": indent_lines(&body, "    "),
            }),
        )?;
        out.push(rendered);
    }

    Ok(out.join("\n"))
}
