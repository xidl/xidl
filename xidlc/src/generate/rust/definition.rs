use crate::error::IdlcResult;
use crate::generate::rust::bitset_dcl::render_bitset_with_config;
use crate::generate::rust::struct_dcl::render_struct_with_config;
use crate::generate::rust::type_dcl::render_typedef_with_config;
use crate::generate::rust::union_def::render_union_with_config;
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use std::collections::HashMap;
use xidl_parser::hir;

const RUST_ITEM_ALLOW_ATTRS: &[&str] = &[
    "allow(unused_imports)",
    "allow(non_upper_case_globals)",
    "allow(non_snake_case)",
    "allow(unused_variables)",
    "allow(unreachable_patterns)",
];

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
) -> IdlcResult<Vec<String>> {
    DefinitionRenderContext::root(renderer).render_module_body(defs)
}

struct DefinitionRenderContext<'a> {
    renderer: &'a RustRenderer,
    module_path: Vec<String>,
}

impl<'a> DefinitionRenderContext<'a> {
    fn root(renderer: &'a RustRenderer) -> Self {
        Self {
            renderer,
            module_path: Vec::new(),
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
                    out.extend(
                        rendered
                            .source
                            .into_iter()
                            .map(wrap_with_generated_allow_attrs),
                    );
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

fn wrap_with_generated_allow_attrs(source: String) -> String {
    let source = source.trim_start_matches(char::is_whitespace);
    let mut out = String::new();
    for attr in RUST_ITEM_ALLOW_ATTRS {
        out.push_str("#[");
        out.push_str(attr);
        out.push_str("]\n");
    }
    out.push_str(source);
    out
}
