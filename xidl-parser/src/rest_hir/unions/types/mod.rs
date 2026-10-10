use super::has_annotation;
use crate::error::{ParseError, ParserResult};
use crate::hir;

mod runtime;

struct Declaration<'a> {
    module_path: Vec<String>,
    ident: &'a str,
    kind: DeclarationKind<'a>,
}

enum DeclarationKind<'a> {
    BuiltinEnum(&'a hir::EnumDcl),
    HttpUnion(&'a hir::UnionDef),
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

    pub(super) fn add_builtins(&mut self, definitions: &'a [hir::Definition]) -> ParserResult<()> {
        for definition in definitions {
            if let hir::Definition::TypeDcl(hir::TypeDcl::ConstrTypeDcl(
                hir::ConstrTypeDcl::EnumDcl(enumeration),
            )) = definition
            {
                if self
                    .0
                    .iter()
                    .any(|decl| decl.module_path.is_empty() && decl.ident == enumeration.ident)
                {
                    return Err(ParseError::Message(format!(
                        "{} is a built-in root declaration when using @http unions; remove the user declaration",
                        enumeration.ident
                    )));
                }
                self.declare(
                    &enumeration.ident,
                    DeclarationKind::BuiltinEnum(enumeration),
                    &[],
                );
            }
        }
        Ok(())
    }

    pub(super) fn http_unions(&self) -> impl Iterator<Item = (&[String], &hir::UnionDef)> {
        self.0.iter().filter_map(|decl| match decl.kind {
            DeclarationKind::HttpUnion(union) => Some((decl.module_path.as_slice(), union)),
            _ => None,
        })
    }

    fn declare(&mut self, ident: &'a str, kind: DeclarationKind<'a>, scope: &[String]) {
        self.0.push(Declaration {
            module_path: scope.to_vec(),
            ident,
            kind,
        });
    }

    fn collect_scope(&mut self, definitions: &'a [hir::Definition], scope: &[String]) {
        for definition in definitions {
            match definition {
                hir::Definition::ModuleDcl(module) => {
                    self.declare(&module.ident, DeclarationKind::Other, scope);
                    let mut nested = scope.to_vec();
                    nested.push(module.ident.clone());
                    self.collect_scope(&module.definition, &nested);
                }
                hir::Definition::TypeDcl(ty) => self.collect_type(ty, scope),
                hir::Definition::ConstrTypeDcl(ty) => self.collect_constructed(ty, scope),
                hir::Definition::ConstDcl(value) => {
                    self.declare(&value.ident, DeclarationKind::Other, scope)
                }
                hir::Definition::ExceptDcl(value) => {
                    self.declare(&value.ident, DeclarationKind::Type, scope)
                }
                hir::Definition::InterfaceDcl(value) => {
                    let ident = match &value.decl {
                        hir::InterfaceDclInner::InterfaceDef(def) => &def.header.ident,
                        hir::InterfaceDclInner::InterfaceForwardDcl(def) => &def.ident,
                    };
                    self.declare(ident, DeclarationKind::Other, scope);
                    if let hir::InterfaceDclInner::InterfaceDef(def) = &value.decl
                        && let Some(body) = &def.interface_body
                    {
                        let mut nested = scope.to_vec();
                        nested.push(ident.clone());
                        for export in &body.0 {
                            match export {
                                hir::Export::TypeDcl(ty) => self.collect_type(ty, &nested),
                                hir::Export::ConstDcl(value) => {
                                    self.declare(&value.ident, DeclarationKind::Other, &nested)
                                }
                                hir::Export::ExceptDcl(value) => {
                                    self.declare(&value.ident, DeclarationKind::Type, &nested)
                                }
                                _ => {}
                            }
                        }
                    }
                }
                hir::Definition::Pragma(_) => {}
            }
        }
    }

    fn collect_type(&mut self, ty: &'a hir::TypeDcl, scope: &[String]) {
        match ty {
            hir::TypeDcl::ConstrTypeDcl(ty) => self.collect_constructed(ty, scope),
            hir::TypeDcl::TypedefDcl(ty) => {
                if let hir::TypedefType::ConstrTypeDcl(ty) = &ty.ty {
                    self.collect_constructed(ty, scope);
                }
                for decl in &ty.decl {
                    let ident = match decl {
                        hir::Declarator::SimpleDeclarator(name) => &name.0,
                        hir::Declarator::ArrayDeclarator(array) => &array.ident,
                    };
                    self.declare(ident, DeclarationKind::Type, scope);
                }
            }
            hir::TypeDcl::NativeDcl(native) => {
                self.declare(&native.decl.0, DeclarationKind::Type, scope)
            }
        }
    }

    fn collect_constructed(&mut self, ty: &'a hir::ConstrTypeDcl, scope: &[String]) {
        let kind = if let hir::ConstrTypeDcl::UnionDef(union) = ty
            && has_annotation(&union.annotations, "http")
        {
            DeclarationKind::HttpUnion(union)
        } else {
            DeclarationKind::Type
        };
        self.declare(ty.ident(), kind, scope);
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
            DeclarationKind::BuiltinEnum(enumeration) => Some(enumeration),
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
