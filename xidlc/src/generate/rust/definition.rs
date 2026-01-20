use crate::error::IdlcResult;
use crate::generate::rust::struct_dcl::render_struct_with_config;
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
            hir::Definition::Pragma(_) => Ok(RustRenderOutput::default()),
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
    let mut module_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut config = hir::SerializeConfig::default();

    for def in defs {
        match def {
            hir::Definition::Pragma(pragma) => {
                config.apply_pragma(*pragma);
            }
            hir::Definition::ModuleDcl(module) => {
                let mut module_config = config;
                let entry = module_map.entry(module.ident.clone()).or_insert_with(|| {
                    module_order.push(module.ident.clone());
                    Vec::new()
                });
                let defs = module.definition.iter().collect::<Vec<_>>();
                let body = render_module_body_with_config(&defs, renderer, &mut module_config)?;
                entry.push(body);
            }
            other => {
                let rendered = render_definition_with_config(other, renderer, &mut config)?;
                out.extend(rendered.source);
            }
        }
    }

    for name in module_order {
        let modules = module_map.remove(&name).unwrap_or_default();
        let body = modules.join("\n");
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

fn render_module_body_with_config(
    defs: &[&hir::Definition],
    renderer: &RustRenderer,
    config: &mut hir::SerializeConfig,
) -> IdlcResult<String> {
    let mut out = Vec::new();
    let mut module_order = Vec::new();
    let mut module_map: HashMap<String, Vec<String>> = HashMap::new();

    for def in defs {
        match def {
            hir::Definition::Pragma(pragma) => {
                config.apply_pragma(*pragma);
            }
            hir::Definition::ModuleDcl(module) => {
                let mut module_config = *config;
                let entry = module_map.entry(module.ident.clone()).or_insert_with(|| {
                    module_order.push(module.ident.clone());
                    Vec::new()
                });
                let defs = module.definition.iter().collect::<Vec<_>>();
                let body = render_module_body_with_config(&defs, renderer, &mut module_config)?;
                entry.push(body);
            }
            other => {
                let rendered = render_definition_with_config(other, renderer, config)?;
                out.extend(rendered.source);
            }
        }
    }

    for name in module_order {
        let modules = module_map.remove(&name).unwrap_or_default();
        let body = modules.join("\n");
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

fn render_definition_with_config(
    def: &hir::Definition,
    renderer: &RustRenderer,
    config: &mut hir::SerializeConfig,
) -> IdlcResult<RustRenderOutput> {
    match def {
        hir::Definition::Pragma(pragma) => {
            config.apply_pragma(*pragma);
            Ok(RustRenderOutput::default())
        }
        hir::Definition::ConstrTypeDcl(constr) => {
            render_constr_with_config(constr, renderer, config)
        }
        hir::Definition::TypeDcl(type_dcl) => {
            render_type_dcl_with_config(type_dcl, renderer, config)
        }
        _ => def.render(renderer),
    }
}

fn render_constr_with_config(
    constr: &hir::ConstrTypeDcl,
    renderer: &RustRenderer,
    config: &hir::SerializeConfig,
) -> IdlcResult<RustRenderOutput> {
    match constr {
        hir::ConstrTypeDcl::StructDcl(def) => render_struct_with_config(def, renderer, config),
        _ => constr.render(renderer),
    }
}

fn render_type_dcl_with_config(
    def: &hir::TypeDcl,
    renderer: &RustRenderer,
    config: &hir::SerializeConfig,
) -> IdlcResult<RustRenderOutput> {
    match &def.decl {
        hir::TypeDclInner::ConstrTypeDcl(constr) => {
            render_constr_with_config(constr, renderer, config)
        }
        _ => def.render(renderer),
    }
}
