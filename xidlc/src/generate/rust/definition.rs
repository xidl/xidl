use crate::error::IdlcResult;
use crate::generate::rust::bitset_dcl::render_bitset_with_config;
use crate::generate::rust::struct_dcl::render_struct_with_config;
use crate::generate::rust::type_dcl::render_typedef_with_config;
use crate::generate::rust::union_def::render_union_with_config;
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use std::collections::HashMap;
use xidl_parser::hir;

impl RustRender for hir::ModuleDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let defs = self.definition.iter().collect::<Vec<_>>();
        let definitions = render_module_body(&defs, renderer)?;
        let rendered = renderer.render_module(&self.ident, &definitions)?;
        Ok(RustRenderOutput::default().push_source(rendered))
    }
}

impl RustRender for hir::Definition {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        DefinitionRenderContext::root(renderer).render_definition(self)
    }
}

pub(crate) fn render_module_body(
    defs: &[&hir::Definition],
    renderer: &RustRenderer,
) -> IdlcResult<Vec<String>> {
    DefinitionRenderContext::root(renderer).render_module_body(defs)
}

/// Renders definitions while tracking the module they are declared in, so
/// renderers that resolve paths see the module the output lands in.
pub(crate) struct DefinitionRenderContext<'a> {
    renderer: &'a RustRenderer,
    module_path: Vec<String>,
}

impl<'a> DefinitionRenderContext<'a> {
    fn root(renderer: &'a RustRenderer) -> Self {
        Self::at(renderer, &[])
    }

    /// A context for definitions rendered inside `module_path`.
    pub(crate) fn at(renderer: &'a RustRenderer, module_path: &[String]) -> Self {
        Self {
            renderer,
            module_path: module_path.to_vec(),
        }
    }

    fn render_module_body(&mut self, defs: &[&hir::Definition]) -> IdlcResult<Vec<String>> {
        let mut out = Vec::new();
        let mut module_order = Vec::new();
        let mut module_map: HashMap<String, Vec<Vec<String>>> = HashMap::new();

        for def in defs {
            match def {
                hir::Definition::Pragma(_) => {}
                hir::Definition::ModuleDcl(module) => {
                    let entry = module_map.entry(module.ident.clone()).or_insert_with(|| {
                        module_order.push(module.ident.clone());
                        Vec::new()
                    });
                    let defs = module.definition.iter().collect::<Vec<_>>();
                    entry.push(self.for_module(&module.ident).render_module_body(&defs)?);
                }
                other => {
                    let rendered = self.render_definition(other)?;
                    out.extend(rendered.source.into_iter());
                }
            }
        }

        for name in module_order {
            let modules = module_map.remove(&name).unwrap_or_default();
            let definitions = modules.into_iter().flatten().collect::<Vec<_>>();
            out.push(self.renderer.render_module(&name, &definitions)?);
        }

        Ok(out)
    }

    fn for_module(&self, ident: &str) -> Self {
        let mut module_path = self.module_path.clone();
        module_path.push(ident.to_string());
        Self {
            renderer: self.renderer,
            module_path,
        }
    }

    pub(crate) fn render_definition(&self, def: &hir::Definition) -> IdlcResult<RustRenderOutput> {
        match def {
            hir::Definition::Pragma(_) => Ok(RustRenderOutput::empty()),
            hir::Definition::ModuleDcl(module) => module.render(self.renderer),
            hir::Definition::ConstrTypeDcl(constr) => self.render_constr(constr),
            hir::Definition::TypeDcl(type_dcl) => self.render_type_dcl(type_dcl),
            hir::Definition::ConstDcl(const_dcl) => const_dcl.render(self.renderer),
            hir::Definition::ExceptDcl(except_dcl) => except_dcl.render(self.renderer),
            hir::Definition::InterfaceDcl(interface) => interface.render(self.renderer),
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
