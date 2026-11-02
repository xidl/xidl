use xidl_parser::hir::{self, DeclarationKind, DeclarationNames};

/// Selects one real declaration while retaining its lexical owners.
pub(super) struct DeclarationSelection<'a> {
    /// Source identity without the compiler's version wrapper.
    pub path: &'a [&'a str],
}

impl DeclarationSelection<'_> {
    /// Finds the declaration at the canonical source path.
    pub fn find(&self, definitions: &[hir::Definition]) -> Option<hir::Definition> {
        let mut forward = None;
        for definition in definitions {
            if let Some(selected) = self.definition(definition) {
                if Self::is_forward(&selected) {
                    forward.get_or_insert(selected);
                } else {
                    return Some(selected);
                }
            }
        }
        forward
    }

    fn is_forward(definition: &hir::Definition) -> bool {
        match definition {
            hir::Definition::TypeDcl(declaration) => Self::forward_type(declaration),
            hir::Definition::ModuleDcl(module) => module.definition.iter().any(Self::is_forward),
            hir::Definition::InterfaceDcl(interface) => {
                match &interface.decl {
                    hir::InterfaceDclInner::InterfaceDef(definition) => {
                        definition.interface_body.as_ref().is_some_and(|body| {
                            body.0.iter().any(|export| matches!(export,
                            hir::Export::TypeDcl(declaration) if Self::forward_type(declaration)
                        ))
                        })
                    }
                    _ => true,
                }
            }
            _ => false,
        }
    }

    fn forward_type(declaration: &hir::TypeDcl) -> bool {
        matches!(
            declaration,
            hir::TypeDcl::ConstrTypeDcl(
                hir::ConstrTypeDcl::StructForwardDcl(_) | hir::ConstrTypeDcl::UnionForwardDcl(_)
            )
        )
    }

    fn definition(&self, definition: &hir::Definition) -> Option<hir::Definition> {
        let (name, remaining) = self.path.split_first()?;
        match definition {
            hir::Definition::ModuleDcl(module) if module.ident == *name => {
                let child = Self { path: remaining }.find(&module.definition)?;
                let mut module = module.clone();
                module.definition = vec![child];
                Some(hir::Definition::ModuleDcl(module))
            }
            hir::Definition::InterfaceDcl(interface) => self.interface(interface),
            _ if remaining.is_empty() => Self::named_type(definition, name),
            _ => None,
        }
    }

    fn named_type(definition: &hir::Definition, name: &str) -> Option<hir::Definition> {
        let mut found = false;
        DeclarationNames::visit_declarations(
            std::slice::from_ref(definition),
            "",
            &mut |path, kind| {
                found |= kind == DeclarationKind::Type && path == name;
            },
        );
        if !found {
            return None;
        }
        let mut result = definition.clone();
        if let hir::Definition::TypeDcl(hir::TypeDcl::TypedefDcl(alias)) = &mut result {
            if let hir::TypedefType::ConstrTypeDcl(constructor) = &alias.ty {
                let constructor =
                    hir::Definition::TypeDcl(hir::TypeDcl::ConstrTypeDcl(constructor.clone()));
                if let Some(selected) = Self::named_type(&constructor, name) {
                    return Some(selected);
                }
            }
            alias.decl.retain(|declaration| match declaration {
                hir::Declarator::SimpleDeclarator(declaration) => declaration.0 == name,
                hir::Declarator::ArrayDeclarator(declaration) => declaration.ident == name,
            });
        }
        Some(result)
    }

    fn interface(&self, interface: &hir::InterfaceDcl) -> Option<hir::Definition> {
        let [scope, name] = self.path else {
            return None;
        };
        let hir::InterfaceDclInner::InterfaceDef(definition) = &interface.decl else {
            return None;
        };
        if definition.header.ident != *scope {
            return None;
        }
        let export = definition
            .interface_body
            .as_ref()?
            .0
            .iter()
            .filter_map(|export| Self::export(export, name))
            .min_by_key(|export| {
                matches!(export, hir::Export::TypeDcl(declaration) if Self::forward_type(declaration))
            })?;
        let mut result = interface.clone();
        if let hir::InterfaceDclInner::InterfaceDef(definition) = &mut result.decl {
            definition.interface_body = Some(hir::InterfaceBody(vec![export]));
        }
        Some(hir::Definition::InterfaceDcl(result))
    }

    fn export(export: &hir::Export, name: &str) -> Option<hir::Export> {
        match export {
            hir::Export::OpDcl(operation) if operation.ident == name => Some(export.clone()),
            hir::Export::AttrDcl(attribute) => {
                Self::attribute(attribute, name).map(hir::Export::AttrDcl)
            }
            hir::Export::TypeDcl(declaration) => {
                match Self::named_type(&hir::Definition::TypeDcl(declaration.clone()), name)? {
                    hir::Definition::TypeDcl(declaration) => {
                        Some(hir::Export::TypeDcl(declaration))
                    }
                    _ => None,
                }
            }
            hir::Export::ExceptDcl(declaration) if declaration.ident == name => {
                Some(export.clone())
            }
            _ => None,
        }
    }

    fn attribute(attribute: &hir::AttrDcl, name: &str) -> Option<hir::AttrDcl> {
        let mut result = attribute.clone();
        let found = match &mut result.decl {
            hir::AttrDclInner::ReadonlyAttrSpec(attribute) => match &mut attribute.declarator {
                hir::ReadonlyAttrDeclarator::SimpleDeclarator(names) => {
                    names.retain(|declaration| declaration.0 == name);
                    !names.is_empty()
                }
                hir::ReadonlyAttrDeclarator::WithRaises { declarator, .. } => declarator.0 == name,
            },
            hir::AttrDclInner::AttrSpec(attribute) => match &mut attribute.declarator {
                hir::AttrDeclarator::SimpleDeclarator(names) => {
                    names.retain(|declaration| declaration.0 == name);
                    !names.is_empty()
                }
                hir::AttrDeclarator::WithRaises { declarator, .. } => declarator.0 == name,
            },
        };
        found.then_some(result)
    }
}
