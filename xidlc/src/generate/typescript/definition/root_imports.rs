//! File-level namespace bindings for absolute IDL references.

use xidl_parser::hir;

use crate::generate::typescript::TsMode;

/// Root namespace bindings needed by the rendered model files.
#[derive(Default)]
pub(super) struct RootImports {
    pub(super) types: bool,
    pub(super) zod: bool,
}

impl RootImports {
    /// Inspects only declarations emitted by the selected render mode.
    pub(super) fn collect(spec: &hir::Specification, mode: TsMode) -> Self {
        let mut imports = Self::default();
        imports.definitions(&spec.0, mode);
        imports
    }

    fn definitions(&mut self, definitions: &[hir::Definition], mode: TsMode) {
        for definition in definitions {
            match definition {
                hir::Definition::ModuleDcl(module) => self.definitions(&module.definition, mode),
                hir::Definition::TypeDcl(ty) if mode.allows_types() => self.type_dcl(ty),
                hir::Definition::ConstrTypeDcl(constr) if mode.allows_types() => {
                    self.constr(constr, false)
                }
                hir::Definition::ExceptDcl(except) if mode.allows_types() => {
                    self.members(&except.member)
                }
                hir::Definition::InterfaceDcl(interface) if mode.allows_interfaces() => {
                    self.interface(interface)
                }
                _ => {}
            }
        }
    }

    fn interface(&mut self, interface: &hir::InterfaceDcl) {
        let hir::InterfaceDclInner::InterfaceDef(definition) = &interface.decl else {
            return;
        };
        let Some(body) = &definition.interface_body else {
            return;
        };
        for export in &body.0 {
            match export {
                hir::Export::OpDcl(operation) => {
                    if let hir::OpTypeSpec::TypeSpec(ty) = &operation.ty {
                        self.reference(Self::ty(ty));
                    }
                    if let Some(parameters) = &operation.parameter {
                        for parameter in &parameters.0 {
                            self.reference(Self::ty(&parameter.ty));
                        }
                    }
                }
                hir::Export::AttrDcl(attribute) => match &attribute.decl {
                    hir::AttrDclInner::ReadonlyAttrSpec(value) => {
                        self.reference(Self::ty(&value.ty))
                    }
                    hir::AttrDclInner::AttrSpec(value) => self.reference(Self::ty(&value.ty)),
                },
                _ => {}
            }
        }
    }

    fn type_dcl(&mut self, ty: &hir::TypeDcl) {
        match ty {
            hir::TypeDcl::ConstrTypeDcl(constr) => self.constr(constr, false),
            hir::TypeDcl::TypedefDcl(alias) => match &alias.ty {
                hir::TypedefType::TypeSpec(ty) => self.reference(Self::ty(ty)),
                hir::TypedefType::ConstrTypeDcl(constr) => self.constr(constr, true),
            },
            hir::TypeDcl::NativeDcl(_) => {}
        }
    }

    fn constr(&mut self, constr: &hir::ConstrTypeDcl, inline: bool) {
        match constr {
            hir::ConstrTypeDcl::StructDcl(value) => {
                self.members(&value.member);
                self.types |= value.parent.iter().any(|name| name.is_root);
                let inherited = value
                    .inherited
                    .iter()
                    .filter(|member| !hir::is_skipped(&member.annotations))
                    .any(|member| Self::ty(&member.ty));
                self.zod |= inherited;
                self.types |= inline && inherited;
            }
            hir::ConstrTypeDcl::UnionDef(value) => {
                for case in &value.case {
                    match &case.element.ty {
                        hir::ElementSpecTy::TypeSpec(ty) => self.reference(Self::ty(ty)),
                        hir::ElementSpecTy::ConstrTypeDcl(constr) => self.constr(constr, true),
                    }
                }
            }
            _ => {}
        }
    }

    fn members(&mut self, members: &[hir::Member]) {
        self.reference(
            members
                .iter()
                .filter(|member| !hir::is_skipped(&member.annotations))
                .any(|member| Self::ty(&member.ty)),
        );
    }

    fn reference(&mut self, rooted: bool) {
        self.types |= rooted;
        self.zod |= rooted;
    }

    fn ty(ty: &hir::TypeSpec) -> bool {
        match ty {
            hir::TypeSpec::ScopedName(name) => name.is_root,
            hir::TypeSpec::SequenceType(sequence) => Self::ty(&sequence.ty),
            hir::TypeSpec::MapType(map) => Self::ty(&map.key) || Self::ty(&map.value),
            hir::TypeSpec::TemplateType(template) => template.args.iter().any(Self::ty),
            _ => false,
        }
    }
}
