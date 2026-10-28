use crate::error::IdlcError;
use crate::error::IdlcResult;
use crate::generate::rust::bitset_dcl::render_bitset_with_config;
use crate::generate::rust::struct_dcl::render_struct_with_config;
use crate::generate::rust::type_dcl::render_typedef_with_config;
use crate::generate::rust::union_def::render_union_with_config;
use crate::generate::rust::util::rust_deprecated_attr;
use crate::generate::rust::util::rust_ident;
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use std::collections::HashMap;
use xidl_parser::hir;

impl RustRender for hir::ModuleDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let defs = self.definition.iter().collect::<Vec<_>>();
        let definitions = render_module_body(&defs, renderer)?;
        let mut attrs = rust_deprecated_attr(&[&self.annotations])?
            .into_iter()
            .collect::<Vec<_>>();
        if !attrs.is_empty() {
            attrs.push("allow(deprecated)".to_string());
        }
        let rendered = renderer.render_module(&self.ident, &definitions, &attrs)?;
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
            hir::Definition::InterfaceDcl(interface) => Err(unsupported_interface_error(interface)),
            hir::Definition::Pragma(_) => Ok(RustRenderOutput::empty()),
        }
    }
}

/// Returns an error because the legacy Rust target emits DDS-style request/reply
/// plumbing for interfaces, which is no longer supported.
fn unsupported_interface_error(interface: &hir::InterfaceDcl) -> IdlcError {
    let ident = match &interface.decl {
        hir::InterfaceDclInner::InterfaceForwardDcl(forward) => rust_ident(&forward.ident),
        hir::InterfaceDclInner::InterfaceDef(def) => rust_ident(&def.header.ident),
    };
    IdlcError::fmt(format!(
        "rust target does not support the `interface {ident}` declaration; use the rust-axum target for HTTP services"
    ))
}

pub(crate) fn render_module_body(
    defs: &[&hir::Definition],
    renderer: &RustRenderer,
) -> IdlcResult<Vec<String>> {
    DefinitionRenderContext::root(renderer).render_module_body(defs)
}

struct DefinitionRenderContext<'a> {
    renderer: &'a RustRenderer,
    module_path: Vec<String>,
    /// Deprecated attributes of merged module blocks, by module name.
    module_attrs: HashMap<String, Vec<String>>,
}

impl<'a> DefinitionRenderContext<'a> {
    fn root(renderer: &'a RustRenderer) -> Self {
        Self {
            module_attrs: HashMap::new(),
            renderer,
            module_path: Vec::new(),
        }
    }

    fn render_module_body(&mut self, defs: &[&hir::Definition]) -> IdlcResult<Vec<String>> {
        let mut out = Vec::new();
        let mut module_order = Vec::new();
        // Reopened modules merge into one block, and their definitions are
        // rendered together so nested reopened modules merge as well.
        let mut module_defs: HashMap<String, Vec<&hir::Definition>> = HashMap::new();

        for def in defs {
            match def {
                hir::Definition::Pragma(_) => {}
                hir::Definition::ModuleDcl(module) => {
                    module_defs
                        .entry(module.ident.clone())
                        .or_insert_with(|| {
                            module_order.push(module.ident.clone());
                            Vec::new()
                        })
                        .extend(module.definition.iter());
                    if let Some(attr) = rust_deprecated_attr(&[&module.annotations])? {
                        self.module_attrs
                            .entry(module.ident.clone())
                            .or_insert_with(|| vec![attr, "allow(deprecated)".to_string()]);
                    }
                }
                other => {
                    let rendered = self.render_definition(other)?;
                    out.extend(rendered.source.into_iter());
                }
            }
        }

        for name in module_order {
            let defs = module_defs.remove(&name).unwrap_or_default();
            let definitions = self.for_module(&name).render_module_body(&defs)?;
            let attrs = self.module_attrs.remove(&name).unwrap_or_default();
            out.push(self.renderer.render_module(&name, &definitions, &attrs)?);
        }

        Ok(out)
    }

    fn for_module(&self, ident: &str) -> Self {
        let mut module_path = self.module_path.clone();
        module_path.push(ident.to_string());
        Self {
            renderer: self.renderer,
            module_path,
            module_attrs: HashMap::new(),
        }
    }

    fn render_definition(&mut self, def: &hir::Definition) -> IdlcResult<RustRenderOutput> {
        match def {
            hir::Definition::Pragma(_) => Ok(RustRenderOutput::empty()),
            hir::Definition::ConstrTypeDcl(constr) => self.render_constr(constr),
            hir::Definition::TypeDcl(type_dcl) => self.render_type_dcl(type_dcl),
            _ => def.render(self.renderer),
        }
    }

    fn render_constr(&self, constr: &hir::ConstrTypeDcl) -> IdlcResult<RustRenderOutput> {
        match constr {
            hir::ConstrTypeDcl::StructDcl(def) => {
                render_struct_with_config(def, self.renderer, &self.module_path)
            }
            hir::ConstrTypeDcl::UnionDef(def) => {
                render_union_with_config(def, self.renderer, &self.module_path)
            }
            hir::ConstrTypeDcl::BitsetDcl(def) => render_bitset_with_config(def, self.renderer),
            hir::ConstrTypeDcl::EnumDcl(def) => def.render(self.renderer),
            hir::ConstrTypeDcl::BitmaskDcl(def) => def.render(self.renderer),
            _ => constr.render(self.renderer),
        }
    }

    fn render_type_dcl(&self, def: &hir::TypeDcl) -> IdlcResult<RustRenderOutput> {
        match def {
            hir::TypeDcl::ConstrTypeDcl(constr) => self.render_constr(constr),
            hir::TypeDcl::TypedefDcl(typedef) => {
                render_typedef_with_config(typedef, self.renderer, &self.module_path)
            }
            hir::TypeDcl::NativeDcl(_) => def.render(self.renderer),
        }
    }
}
