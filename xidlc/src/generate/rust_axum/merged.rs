use crate::error::IdlcResult;
use crate::generate::rust::RustRender;
use crate::generate::rust_axum::interface::render_interface_with_path;
use crate::generate::rust_axum::transport::TypeRegistry;
use crate::generate::rust_axum::{RustAxumRenderOutput, RustAxumRenderer};
use std::collections::HashMap;
use xidl_parser::hir;

/// Merged Axum specification renderer with types before interfaces.
pub(crate) struct MergedAxumSpecBuilder<'a> {
    axum_renderer: &'a RustAxumRenderer,
    registry: TypeRegistry,
}

impl<'a> MergedAxumSpecBuilder<'a> {
    /// Create a merged renderer from prepared Axum and Rust renderers.
    pub(crate) fn new(axum_renderer: &'a RustAxumRenderer, registry: TypeRegistry) -> Self {
        Self {
            axum_renderer,
            registry,
        }
    }

    /// Render all declared models before interfaces.
    pub(crate) fn render(&self, spec: &hir::Specification) -> IdlcResult<Vec<String>> {
        let defs = spec.0.iter().collect::<Vec<_>>();
        self.render_body(&defs, &[])
    }

    fn render_body(
        &self,
        defs: &[&hir::Definition],
        module_path: &[String],
    ) -> IdlcResult<Vec<String>> {
        let mut out = Vec::new();
        for &def in defs {
            if self.is_type_def(def) {
                out.extend(self.render_type(def, module_path)?.source);
            }
        }
        for &def in defs {
            if let hir::Definition::InterfaceDcl(interface) = def {
                let rendered = render_interface_with_path(
                    interface,
                    self.axum_renderer,
                    module_path,
                    &self.registry,
                )?;
                out.extend(rendered.source);
            }
        }
        out.extend(self.render_modules(defs, module_path)?);
        Ok(out)
    }

    fn render_modules(
        &self,
        defs: &[&hir::Definition],
        module_path: &[String],
    ) -> IdlcResult<Vec<String>> {
        let mut out = Vec::new();
        let mut order = Vec::new();
        let mut grouped: HashMap<String, Vec<&hir::ModuleDcl>> = HashMap::new();
        for &def in defs {
            if let hir::Definition::ModuleDcl(module) = def {
                grouped
                    .entry(module.ident.clone())
                    .or_insert_with(|| {
                        order.push(module.ident.clone());
                        Vec::new()
                    })
                    .push(module);
            }
        }
        for name in order {
            let modules = grouped.remove(&name).unwrap_or_default();
            let mut inner_defs = Vec::new();
            for module in &modules {
                for def in &module.definition {
                    inner_defs.push(def);
                }
            }
            let mut next_path = module_path.to_vec();
            next_path.push(name.clone());
            let definitions = self.render_body(&inner_defs, &next_path)?;
            let mut attrs = Vec::new();
            for module in &modules {
                if let Some(attr) =
                    crate::generate::rust::util::rust_deprecated_attr(&[&module.annotations])?
                {
                    attrs.push(attr);
                    attrs.push("allow(deprecated)".to_string());
                    break;
                }
            }
            let rendered = self.axum_renderer.render_template(
                "module.rs.j2",
                &serde_json::json!({
                    "ident": crate::generate::rust::util::rust_ident(&name),
                    "definitions": &definitions,
                    "attrs": &attrs,
                }),
            )?;
            out.push(rendered);
        }
        Ok(out)
    }

    fn is_type_def(&self, def: &hir::Definition) -> bool {
        !matches!(
            def,
            hir::Definition::ModuleDcl(_)
                | hir::Definition::InterfaceDcl(_)
                | hir::Definition::Pragma(_)
        )
    }

    fn render_type(
        &self,
        def: &hir::Definition,
        module_path: &[String],
    ) -> IdlcResult<RustAxumRenderOutput> {
        let mut def = def.clone();
        super::scope::TypeScope::new(&self.registry, module_path, &self.axum_renderer.models)
            .lower_definition(&mut def);
        let output = def.render(&self.axum_renderer.models)?;
        Ok(RustAxumRenderOutput {
            source: output.source,
        })
    }
}
