use crate::error::{ParseError, ParserResult};
use crate::hir;

struct Declaration<'a> {
    module_path: Vec<String>,
    ident: &'a str,
    kind: DeclarationKind<'a>,
}

enum DeclarationKind<'a> {
    Enum(&'a hir::EnumDcl),
    Type,
    Other,
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
                    self.0.push(Declaration {
                        module_path: scope.to_vec(),
                        ident: &module.ident,
                        kind: DeclarationKind::Other,
                    });
                    let mut nested = scope.to_vec();
                    nested.push(module.ident.clone());
                    self.collect_scope(&module.definition, &nested);
                }
                hir::Definition::TypeDcl(hir::TypeDcl::ConstrTypeDcl(ty))
                | hir::Definition::ConstrTypeDcl(ty) => self.collect_constructed(ty, scope),
                hir::Definition::TypeDcl(hir::TypeDcl::TypedefDcl(ty)) => {
                    if let hir::TypedefType::ConstrTypeDcl(ty) = &ty.ty {
                        self.collect_constructed(ty, scope);
                    }
                    for decl in &ty.decl {
                        let ident = match decl {
                            hir::Declarator::SimpleDeclarator(s) => &s.0,
                            hir::Declarator::ArrayDeclarator(a) => &a.ident,
                        };
                        self.0.push(Declaration {
                            module_path: scope.to_vec(),
                            ident,
                            kind: DeclarationKind::Type,
                        });
                    }
                }
                hir::Definition::TypeDcl(hir::TypeDcl::NativeDcl(native)) => {
                    self.0.push(Declaration {
                        module_path: scope.to_vec(),
                        ident: &native.decl.0,
                        kind: DeclarationKind::Type,
                    });
                }
                hir::Definition::ConstDcl(value) => self.0.push(Declaration {
                    module_path: scope.to_vec(),
                    ident: &value.ident,
                    kind: DeclarationKind::Other,
                }),
                hir::Definition::ExceptDcl(value) => self.0.push(Declaration {
                    module_path: scope.to_vec(),
                    ident: &value.ident,
                    kind: DeclarationKind::Type,
                }),
                hir::Definition::InterfaceDcl(value) => self.0.push(Declaration {
                    module_path: scope.to_vec(),
                    ident: match &value.decl {
                        hir::InterfaceDclInner::InterfaceDef(def) => &def.header.ident,
                        hir::InterfaceDclInner::InterfaceForwardDcl(def) => &def.ident,
                    },
                    kind: DeclarationKind::Other,
                }),
                _ => {}
            }
        }
    }

    fn collect_constructed(&mut self, ty: &'a hir::ConstrTypeDcl, scope: &[String]) {
        self.0.push(Declaration {
            module_path: scope.to_vec(),
            ident: ty.ident(),
            kind: if let hir::ConstrTypeDcl::EnumDcl(v) = ty {
                DeclarationKind::Enum(v)
            } else {
                DeclarationKind::Type
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

    pub(super) fn content_type(
        &self,
        name: &hir::ScopedName,
        scope: &[String],
    ) -> Option<&'a hir::EnumDcl> {
        let declaration = self.resolve(name, scope)?;
        match declaration.kind {
            DeclarationKind::Enum(enumeration)
                if declaration.module_path.is_empty() && declaration.ident == "ContentType" =>
            {
                Some(enumeration)
            }
            _ => None,
        }
    }

    pub(super) fn content_type_member<'b>(
        &self,
        name: &hir::ScopedName,
        scope: &[String],
        content_type: &'b hir::EnumDcl,
    ) -> Option<&'b hir::Enumerator> {
        let (ident, owner) = name.name.split_last()?;
        if !owner.is_empty() {
            let owner = hir::ScopedName {
                name: owner.to_vec(),
                is_root: name.is_root,
            };
            self.content_type(&owner, scope)?;
        }
        content_type
            .member
            .iter()
            .find(|member| &member.ident == ident)
    }

    pub(super) fn qualify(&self, ty: &mut hir::TypeSpec, scope: &[String]) -> ParserResult<()> {
        match ty {
            hir::TypeSpec::ScopedName(name) => {
                let decl = self
                    .resolve(name, scope)
                    .filter(|decl| !matches!(decl.kind, DeclarationKind::Other))
                    .ok_or_else(|| {
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
