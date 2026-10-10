use crate::error::{ParseError, ParserResult};
use crate::hir;

struct Declaration<'a> {
    module_path: Vec<String>,
    ident: &'a str,
    enum_def: Option<&'a hir::EnumDcl>,
}

/// Borrowed type declarations used while projecting HTTP unions.
pub(super) struct TypeDeclarations<'a>(Vec<Declaration<'a>>);

impl<'a> TypeDeclarations<'a> {
    pub(super) fn collect(definitions: &'a [hir::Definition]) -> Self {
        let mut types = Self(Vec::new());
        types.collect_scope(definitions, &[]);
        types
    }

    fn collect_scope(&mut self, definitions: &'a [hir::Definition], scope: &[String]) {
        for definition in definitions {
            match definition {
                hir::Definition::ModuleDcl(module) => {
                    let mut nested = scope.to_vec();
                    nested.push(module.ident.clone());
                    self.collect_scope(&module.definition, &nested);
                }
                hir::Definition::TypeDcl(hir::TypeDcl::ConstrTypeDcl(ty))
                | hir::Definition::ConstrTypeDcl(ty) => self.collect_constructed(ty, scope),
                hir::Definition::TypeDcl(hir::TypeDcl::TypedefDcl(ty)) => {
                    for decl in &ty.decl {
                        let ident = match decl {
                            hir::Declarator::SimpleDeclarator(s) => &s.0,
                            hir::Declarator::ArrayDeclarator(a) => &a.ident,
                        };
                        self.0.push(Declaration {
                            module_path: scope.to_vec(),
                            ident,
                            enum_def: None,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_constructed(&mut self, ty: &'a hir::ConstrTypeDcl, scope: &[String]) {
        let ident = match ty {
            hir::ConstrTypeDcl::StructDcl(v) => &v.ident,
            hir::ConstrTypeDcl::StructForwardDcl(v) => &v.ident,
            hir::ConstrTypeDcl::UnionDef(v) => &v.ident,
            hir::ConstrTypeDcl::UnionForwardDcl(v) => &v.ident,
            hir::ConstrTypeDcl::EnumDcl(v) => &v.ident,
            hir::ConstrTypeDcl::BitsetDcl(v) => &v.ident,
            hir::ConstrTypeDcl::BitmaskDcl(v) => &v.ident,
        };
        self.0.push(Declaration {
            module_path: scope.to_vec(),
            ident,
            enum_def: if let hir::ConstrTypeDcl::EnumDcl(v) = ty {
                Some(v)
            } else {
                None
            },
        });
    }

    fn resolve(&self, name: &hir::ScopedName, scope: &[String]) -> Option<&Declaration<'a>> {
        super::HttpUnion::resolve_scoped_definition(
            self.0
                .iter()
                .map(|decl| (decl.module_path.as_slice(), decl)),
            scope,
            name,
            |decl| decl.ident,
        )
    }

    pub(super) fn enum_definition(
        &self,
        name: &hir::ScopedName,
        scope: &[String],
    ) -> Option<&'a hir::EnumDcl> {
        self.resolve(name, scope)?.enum_def
    }

    pub(super) fn qualify(&self, ty: &mut hir::TypeSpec, scope: &[String]) -> ParserResult<()> {
        match ty {
            hir::TypeSpec::ScopedName(name) => {
                let decl = self.resolve(name, scope).ok_or_else(|| {
                    ParseError::Message(format!(
                        "HTTP union payload type '{}' does not resolve",
                        name.name.join("::")
                    ))
                })?;
                name.name = decl
                    .module_path
                    .iter()
                    .cloned()
                    .chain(std::iter::once(decl.ident.to_string()))
                    .collect();
                name.is_root = true;
            }
            hir::TypeSpec::SequenceType(seq) => self.qualify(&mut seq.ty, scope)?,
            hir::TypeSpec::MapType(map) => {
                self.qualify(&mut map.key, scope)?;
                self.qualify(&mut map.value, scope)?;
            }
            hir::TypeSpec::TemplateType(t) => {
                for arg in &mut t.args {
                    self.qualify(arg, scope)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
