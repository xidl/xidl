use crate::typed_ast::*;

use super::{Site, check::Checker};

impl Checker {
    pub(super) fn definitions(&mut self, definitions: &[Definition]) {
        for definition in definitions {
            self.definition(definition);
        }
    }

    fn definition(&mut self, definition: &Definition) {
        match definition {
            Definition::ModuleDcl(module) => {
                self.annotations(&module.annotations, Site::Module);
                self.definitions(&module.definition);
            }
            Definition::TypeDcl(ty) => self.type_dcl(ty),
            Definition::ConstDcl(constant) => {
                self.annotations(&constant.annotations, Site::Constant)
            }
            Definition::ExceptDcl(exception) => self.exception(exception),
            Definition::InterfaceDcl(interface) => self.interface(interface),
            Definition::TemplateModuleDcl(template) => {
                for definition in &template.definition {
                    if let TplDefinition::Definition(definition) = definition {
                        self.definition(definition);
                    }
                }
            }
            Definition::TemplateModuleInst(_)
            | Definition::ImportDcl(_)
            | Definition::PreprocInclude(_)
            | Definition::PreprocCall(_)
            | Definition::PreprocDefine(_) => {}
        }
    }

    fn type_dcl(&mut self, ty: &TypeDcl) {
        let site = match &ty.decl {
            TypeDclInner::ConstrTypeDcl(ty) => {
                self.compound(ty);
                match ty {
                    ConstrTypeDcl::StructDcl(_) => Site::Struct,
                    ConstrTypeDcl::UnionDcl(_) => Site::Union,
                    ConstrTypeDcl::EnumDcl(_) => Site::Enum,
                    ConstrTypeDcl::BitsetDcl(_) => Site::Bitset,
                    ConstrTypeDcl::BitmaskDcl(_) => Site::Bitmask,
                }
            }
            TypeDclInner::TypedefDcl(ty) => {
                if let TypeDeclaratorInner::ConstrTypeDcl(ty) = &ty.decl.ty {
                    self.compound(ty);
                }
                Site::Typedef
            }
            TypeDclInner::NativeDcl(_) => Site::Native,
        };
        self.annotations(&ty.annotations, site);
    }

    fn compound(&mut self, ty: &ConstrTypeDcl) {
        match ty {
            ConstrTypeDcl::StructDcl(StructDcl::StructDef(definition)) => {
                self.members(&definition.member);
            }
            ConstrTypeDcl::UnionDcl(UnionDcl::UnionDef(definition)) => {
                for case in &definition.case {
                    self.annotations(&case.element.annotations, Site::UnionMember);
                    if let ElementSpecTy::ConstrTypeDcl(ty) = &case.element.ty {
                        self.compound(ty);
                    }
                }
            }
            ConstrTypeDcl::EnumDcl(definition) => {
                for member in &definition.member {
                    self.annotations(&member.annotations, Site::Enumerator);
                }
            }
            ConstrTypeDcl::BitmaskDcl(definition) => {
                for value in &definition.value {
                    self.annotations(&value.annotations, Site::BitmaskValue);
                }
            }
            ConstrTypeDcl::StructDcl(StructDcl::StructForwardDcl(_))
            | ConstrTypeDcl::UnionDcl(UnionDcl::UnionForwardDcl(_))
            | ConstrTypeDcl::BitsetDcl(_) => {}
        }
    }

    fn members(&mut self, members: &[Member]) {
        for member in members {
            self.annotations(&member.annotations, Site::Member);
        }
    }

    fn exception(&mut self, exception: &ExceptDcl) {
        self.annotations(&exception.annotations, Site::Exception);
        self.members(&exception.member);
    }

    fn interface(&mut self, interface: &InterfaceDcl) {
        self.annotations(&interface.annotations, Site::Interface);
        let InterfaceDclInner::InterfaceDef(definition) = &interface.decl else {
            return;
        };
        let Some(body) = &definition.interface_body else {
            return;
        };
        for export in &body.0 {
            match export {
                Export::TypeDcl(ty) => self.type_dcl(ty),
                Export::ConstDcl(constant) => {
                    self.annotations(&constant.annotations, Site::Constant)
                }
                Export::ExceptDcl(exception) => self.exception(exception),
                Export::AttrDcl(attribute) => {
                    self.annotations(&attribute.annotations, Site::Attribute)
                }
                Export::OpDcl(operation) => {
                    self.annotations(&operation.annotations, Site::Operation);
                    if let Some(parameters) = &operation.parameter {
                        for parameter in &parameters.0 {
                            self.annotations(&parameter.annotations, Site::Parameter);
                        }
                    }
                }
            }
        }
    }
}
