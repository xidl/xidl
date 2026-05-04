use crate::error::IdlcResult;
use crate::generate::rust::util::rust_ident;
use crate::generate::rust_axum::interface::render_interface_with_path;
use crate::generate::rust_axum::transport::{TypeRegistry, build_type_registry};
use crate::generate::rust_axum::{RustAxumRender, RustAxumRenderOutput, RustAxumRenderer};
use serde_json::json;
use std::collections::HashMap;
use xidl_parser::hir;

impl RustAxumRender for hir::ModuleDcl {
    fn render(&self, renderer: &RustAxumRenderer) -> IdlcResult<RustAxumRenderOutput> {
        let defs = self.definition.iter().collect::<Vec<_>>();
        let module_path = vec![self.ident.clone()];
        let registry = build_type_registry(&defs, &module_path);
        let definitions = render_module_body_with_path(&defs, renderer, &module_path, &registry)?;
        let rendered = renderer.render_template(
            "module.rs.j2",
            &json!({
                "ident": rust_ident(&self.ident),
                "definitions": definitions,
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
                let defs = [self];
                let registry = build_type_registry(&defs, &[]);
                render_interface_with_path(interface, renderer, &[], &registry)
            }
            _ => Ok(RustAxumRenderOutput::default()),
        }
    }
}

pub(crate) fn render_module_body(
    defs: &[&hir::Definition],
    renderer: &RustAxumRenderer,
) -> IdlcResult<Vec<String>> {
    let registry = build_type_registry(defs, &[]);
    render_module_body_with_path(defs, renderer, &[], &registry)
}

fn render_module_body_with_path(
    defs: &[&hir::Definition],
    renderer: &RustAxumRenderer,
    module_path: &[String],
    registry: &TypeRegistry,
) -> IdlcResult<Vec<String>> {
    let mut out = Vec::new();
    let mut module_order = Vec::new();
    let mut module_map: HashMap<String, Vec<Vec<String>>> = HashMap::new();

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
                let definitions =
                    render_module_body_with_path(&defs, renderer, &next_path, registry)?;
                entry.push(definitions);
            }
            hir::Definition::InterfaceDcl(interface) => {
                let rendered =
                    render_interface_with_path(interface, renderer, module_path, registry)?;
                out.extend(rendered.source);
            }
            _ => {}
        }
    }

    for name in module_order {
        let modules = module_map.remove(&name).unwrap_or_default();
        let definitions = modules.into_iter().flatten().collect::<Vec<_>>();
        let rendered = renderer.render_template(
            "module.rs.j2",
            &serde_json::json!({
                "ident": crate::generate::rust::util::rust_ident(&name),
                "definitions": &definitions,
            }),
        )?;
        out.push(rendered);
    }

    Ok(out)
}
