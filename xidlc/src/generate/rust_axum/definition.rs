use crate::error::IdlcResult;
use crate::generate::rust::util::rust_ident;
use crate::generate::rust_axum::interface::render_interface_with_path;
use crate::generate::rust_axum::{RustAxumRender, RustAxumRenderOutput, RustAxumRenderer};
use serde_json::json;
use std::collections::HashMap;
use xidl_parser::hir;

impl RustAxumRender for hir::ModuleDcl {
    fn render(&self, renderer: &RustAxumRenderer) -> IdlcResult<RustAxumRenderOutput> {
        let defs = self.definition.iter().collect::<Vec<_>>();
        let module_path = vec![self.ident.clone()];
        let body = render_module_body_with_path(&defs, renderer, &module_path)?;
        let rendered = renderer.render_template(
            "module.rs.j2",
            &json!({
                "ident": rust_ident(&self.ident),
                "body": body,
            }),
        )?;
        Ok(RustAxumRenderOutput {
            source: vec![rendered],
        })
    }
}

impl RustAxumRender for hir::Definition {
    fn render(&self, renderer: &RustAxumRenderer) -> IdlcResult<RustAxumRenderOutput> {
        match self {
            hir::Definition::ModuleDcl(module) => module.render(renderer),
            hir::Definition::InterfaceDcl(interface) => {
                render_interface_with_path(interface, renderer, &[])
            }
            _ => Ok(RustAxumRenderOutput::default()),
        }
    }
}

pub(crate) fn render_module_body(
    defs: &[&hir::Definition],
    renderer: &RustAxumRenderer,
) -> IdlcResult<String> {
    render_module_body_with_path(defs, renderer, &[])
}

fn render_module_body_with_path(
    defs: &[&hir::Definition],
    renderer: &RustAxumRenderer,
    module_path: &[String],
) -> IdlcResult<String> {
    let mut out = Vec::new();
    let mut module_order = Vec::new();
    let mut module_map: HashMap<String, Vec<String>> = HashMap::new();

    for def in defs {
        match def {
            hir::Definition::ModuleDcl(module) => {
                let entry = module_map.entry(module.ident.clone()).or_insert_with(|| {
                    module_order.push(module.ident.clone());
                    Vec::new()
                });
                let defs = module.definition.iter().collect::<Vec<_>>();
                let mut next_path = module_path.to_vec();
                next_path.push(module.ident.clone());
                let body = render_module_body_with_path(&defs, renderer, &next_path)?;
                entry.push(body);
            }
            hir::Definition::InterfaceDcl(interface) => {
                let rendered = render_interface_with_path(interface, renderer, module_path)?;
                out.extend(rendered.source);
            }
            _ => {}
        }
    }

    for name in module_order {
        let modules = module_map.remove(&name).unwrap_or_default();
        let body = modules.join("\n");
        let rendered = renderer.render_template(
            "module.rs.j2",
            &serde_json::json!({
                "ident": crate::generate::rust::util::rust_ident(&name),
                "body": &body,
            }),
        )?;
        out.push(rendered);
    }

    Ok(out.join("\n"))
}
