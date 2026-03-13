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
        let body = render_module_body(&defs, renderer)?;
        let rendered = renderer.render_module(&self.ident, &body)?;
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
            hir::Definition::Pragma(_) => Ok(RustRenderOutput::empty()),
        }
    }
}

pub(crate) fn render_module_body(
    defs: &[&hir::Definition],
    renderer: &RustRenderer,
) -> IdlcResult<String> {
    DefinitionRenderContext::root(renderer).render_module_body(defs)
}

struct DefinitionRenderContext<'a> {
    renderer: &'a RustRenderer,
    config: hir::SerializeConfig,
    module_path: Vec<String>,
}

impl<'a> DefinitionRenderContext<'a> {
    fn root(renderer: &'a RustRenderer) -> Self {
        Self {
            renderer,
            config: hir::SerializeConfig::default(),
            module_path: Vec::new(),
        }
    }

    fn render_module_body(&mut self, defs: &[&hir::Definition]) -> IdlcResult<String> {
        let mut out = Vec::new();
        let mut module_order = Vec::new();
        let mut module_map: HashMap<String, Vec<String>> = HashMap::new();

        for def in defs {
            match def {
                hir::Definition::Pragma(pragma) => {
                    self.config.apply_pragma(pragma.clone());
                }
                hir::Definition::ModuleDcl(module) => {
                    let entry = module_map.entry(module.ident.clone()).or_insert_with(|| {
                        module_order.push(module.ident.clone());
                        Vec::new()
                    });
                    let defs = module.definition.iter().collect::<Vec<_>>();
                    let body = self.for_module(&module.ident).render_module_body(&defs)?;
                    entry.push(body);
                }
                other => {
                    let rendered = self.render_definition(other)?;
                    out.extend(rendered.source);
                }
            }
        }

        for name in module_order {
            let modules = module_map.remove(&name).unwrap_or_default();
            out.push(self.renderer.render_module(&name, &modules.join("\n"))?);
        }

        Ok(out.join("\n"))
    }

    fn for_module(&self, ident: &str) -> Self {
        let mut module_path = self.module_path.clone();
        module_path.push(ident.to_string());
        Self {
            renderer: self.renderer,
            config: self.config,
            module_path,
        }
    }

    fn render_definition(&mut self, def: &hir::Definition) -> IdlcResult<RustRenderOutput> {
        match def {
            hir::Definition::Pragma(pragma) => {
                self.config.apply_pragma(pragma.clone());
                Ok(RustRenderOutput::empty())
            }
            hir::Definition::ConstrTypeDcl(constr) => self.render_constr(constr),
            hir::Definition::TypeDcl(type_dcl) => self.render_type_dcl(type_dcl),
            _ => def.render(self.renderer),
        }
    }

    fn render_constr(&self, constr: &hir::ConstrTypeDcl) -> IdlcResult<RustRenderOutput> {
        match constr {
            hir::ConstrTypeDcl::StructDcl(def) => {
                render_struct_with_config(def, self.renderer, &self.config, &self.module_path)
            }
            hir::ConstrTypeDcl::UnionDef(def) => {
                render_union_with_config(def, self.renderer, &self.config, &self.module_path)
            }
            hir::ConstrTypeDcl::BitsetDcl(def) => {
                render_bitset_with_config(def, self.renderer, &self.config)
            }
            hir::ConstrTypeDcl::EnumDcl(def) => def.render(self.renderer),
            hir::ConstrTypeDcl::BitmaskDcl(def) => def.render(self.renderer),
            _ => constr.render(self.renderer),
        }
    }

    fn render_type_dcl(&self, def: &hir::TypeDcl) -> IdlcResult<RustRenderOutput> {
        match &def.decl {
            hir::TypeDclInner::ConstrTypeDcl(constr) => self.render_constr(constr),
            hir::TypeDclInner::TypedefDcl(typedef) => {
                render_typedef_with_config(typedef, self.renderer, &self.module_path)
            }
            _ => def.render(self.renderer),
        }
    }
}
