use super::transport::TypeRegistry;
use crate::generate::rust::util::{rust_ident, rust_scoped_name};
use crate::generate::utils::scope::resolve_canonical;
use xidl_parser::hir;

/// IDL lookup scope and Rust output scope for one rendering site.
#[derive(Clone, Copy)]
pub(crate) struct TypeScope<'a> {
    pub(crate) registry: &'a TypeRegistry,
    pub(crate) declaration: &'a [String],
    pub(crate) output: &'a [String],
}

impl<'a> TypeScope<'a> {
    pub(crate) fn new(registry: &'a TypeRegistry, module: &'a [String]) -> Self {
        Self {
            registry,
            declaration: module,
            output: module,
        }
    }

    pub(crate) fn in_declaration(self, declaration: &'a [String]) -> Self {
        Self {
            declaration,
            ..self
        }
    }

    pub(crate) fn resolve(self, name: &hir::ScopedName) -> Option<String> {
        resolve_canonical(self.declaration, name, |key| {
            self.registry.contains_key(key)
        })
    }

    pub(crate) fn render(self, name: &hir::ScopedName) -> String {
        self.resolve(name)
            .map(|key| Self::relative_path(self.output, &key))
            .unwrap_or_else(|| rust_scoped_name(name))
    }

    pub(crate) fn relative_path(output: &[String], canonical: &str) -> String {
        let target = canonical.split("::").collect::<Vec<_>>();
        let common = output
            .iter()
            .zip(&target[..target.len() - 1])
            .take_while(|(a, b)| a.as_str() == **b)
            .count();
        std::iter::repeat_n("super".to_string(), output.len() - common)
            .chain(target[common..].iter().map(|part| rust_ident(part)))
            .collect::<Vec<_>>()
            .join("::")
    }

    /// Lower a public declaration before handing it to the plain Rust renderer.
    pub(crate) fn lower_definition(self, def: &mut hir::Definition) {
        match def {
            hir::Definition::TypeDcl(ty) => match ty {
                hir::TypeDcl::ConstrTypeDcl(c) => self.lower_constructed(c),
                hir::TypeDcl::TypedefDcl(t) => match &mut t.ty {
                    hir::TypedefType::TypeSpec(ty) => self.lower_type(ty),
                    hir::TypedefType::ConstrTypeDcl(c) => self.lower_constructed(c),
                },
                _ => {}
            },
            hir::Definition::ConstrTypeDcl(c) => self.lower_constructed(c),
            hir::Definition::ConstDcl(c) => {
                match &mut c.ty {
                    hir::ConstType::ScopedName(name) => self.lower_name(name),
                    hir::ConstType::SequenceType(seq) => self.lower_type(&mut seq.ty),
                    _ => {}
                }
                self.lower_expr(&mut c.value);
            }
            hir::Definition::ExceptDcl(e) => {
                for member in &mut e.member {
                    self.lower_type(&mut member.ty);
                }
            }
            _ => {}
        }
    }

    fn lower_constructed(self, def: &mut hir::ConstrTypeDcl) {
        match def {
            hir::ConstrTypeDcl::StructDcl(s) => {
                for parent in &mut s.parent {
                    self.lower_name(parent);
                }
                for member in &mut s.member {
                    self.lower_type(&mut member.ty);
                }
            }
            hir::ConstrTypeDcl::UnionDef(u) => {
                if let hir::SwitchTypeSpec::ScopedName(name) = &mut u.switch_type_spec {
                    self.lower_name(name);
                }
                for case in &mut u.case {
                    for label in &mut case.label {
                        if let hir::CaseLabel::Value(expr) = label {
                            self.lower_expr(expr);
                        }
                    }
                    match &mut case.element.ty {
                        hir::ElementSpecTy::TypeSpec(ty) => self.lower_type(ty),
                        hir::ElementSpecTy::ConstrTypeDcl(c) => self.lower_constructed(c),
                    }
                }
            }
            hir::ConstrTypeDcl::BitsetDcl(b) => {
                if let Some(parent) = &mut b.parent {
                    self.lower_name(parent);
                }
            }
            _ => {}
        }
    }

    fn lower_type(self, ty: &mut hir::TypeSpec) {
        match ty {
            hir::TypeSpec::ScopedName(name) => self.lower_name(name),
            hir::TypeSpec::SequenceType(seq) => self.lower_type(&mut seq.ty),
            hir::TypeSpec::MapType(map) => {
                self.lower_type(&mut map.key);
                self.lower_type(&mut map.value);
            }
            hir::TypeSpec::TemplateType(t) => {
                for ty in &mut t.args {
                    self.lower_type(ty);
                }
            }
            _ => {}
        }
    }

    fn lower_expr(self, expr: &mut hir::ConstExpr) {
        match expr {
            hir::ConstExpr::ScopedName(name) => {
                // Enum variants share the enum declaration's scope.
                if self.resolve(name).is_none() && name.name.len() > 1 {
                    let mut parent = name.clone();
                    let variant = parent.name.pop();
                    if self.resolve(&parent).is_some() {
                        self.lower_name(&mut parent);
                        parent.name.extend(variant);
                        *name = parent;
                        return;
                    }
                }
                self.lower_name(name);
            }
            hir::ConstExpr::UnaryExpr(_, inner) => self.lower_expr(inner),
            hir::ConstExpr::BinaryExpr(_, left, right) => {
                self.lower_expr(left);
                self.lower_expr(right);
            }
            hir::ConstExpr::Literal(_) => {}
        }
    }

    fn lower_name(self, name: &mut hir::ScopedName) {
        if let Some(canonical) = self.resolve(name) {
            name.name = Self::relative_path(self.output, &canonical)
                .split("::")
                .map(|part| part.strip_prefix("r#").unwrap_or(part).to_string())
                .collect();
            name.is_root = false;
        }
    }
}
