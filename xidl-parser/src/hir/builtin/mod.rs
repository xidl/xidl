use super::{Definition, Specification, TypeDcl};
use crate::error::{ParseError, ParserResult};
use crate::hir;
use crate::rest_hir::semantics::has_annotation;

#[cfg(test)]
mod tests;

/// Source declarations loaded only by documents that use HTTP unions.
pub(super) struct HttpBuiltins;

impl HttpBuiltins {
    pub(super) fn load(spec: &mut Specification) -> ParserResult<()> {
        if !Self::required(&spec.0) {
            return Ok(());
        }
        if spec.0.iter().any(Self::conflicts) {
            return Err(ParseError::Message(
                "ContentType is a built-in root declaration when using @http unions; remove the user declaration"
                    .to_string(),
            ));
        }
        let source = include_str!("../../../builtin/http.idl");
        let typed = crate::parser::parser_text(source)?;
        super::spec::collect_defs_with_context(typed.0, &mut Vec::new(), false, &mut spec.0)?;
        // Validate the annotation for every target, including plain Rust/HIR.
        // HTTP projection later collects the same cases for transport metadata.
        crate::rest_hir::HttpUnion::collect(&spec.0)?;
        Ok(())
    }

    fn required(definitions: &[Definition]) -> bool {
        definitions.iter().any(|definition| match definition {
            Definition::ModuleDcl(module) => Self::required(&module.definition),
            Definition::TypeDcl(TypeDcl::ConstrTypeDcl(hir::ConstrTypeDcl::UnionDef(union)))
            | Definition::ConstrTypeDcl(hir::ConstrTypeDcl::UnionDef(union)) => {
                has_annotation(&union.annotations, "http")
            }
            _ => false,
        })
    }

    fn conflicts(definition: &Definition) -> bool {
        let name: &str = match definition {
            Definition::ModuleDcl(module) => &module.ident,
            Definition::TypeDcl(TypeDcl::ConstrTypeDcl(ty)) | Definition::ConstrTypeDcl(ty) => {
                ty.ident()
            }
            Definition::TypeDcl(TypeDcl::TypedefDcl(ty)) => {
                if let hir::TypedefType::ConstrTypeDcl(ty) = &ty.ty
                    && ty.ident() == "ContentType"
                {
                    return true;
                }
                return ty.decl.iter().any(|decl| match decl {
                    hir::Declarator::SimpleDeclarator(name) => name.0 == "ContentType",
                    hir::Declarator::ArrayDeclarator(array) => array.ident == "ContentType",
                });
            }
            Definition::TypeDcl(TypeDcl::NativeDcl(native)) => &native.decl.0,
            Definition::ConstDcl(value) => &value.ident,
            Definition::ExceptDcl(value) => &value.ident,
            Definition::InterfaceDcl(interface) => match &interface.decl {
                hir::InterfaceDclInner::InterfaceDef(value) => &value.header.ident,
                hir::InterfaceDclInner::InterfaceForwardDcl(value) => &value.ident,
            },
            Definition::Pragma(_) => return false,
        };
        name == "ContentType"
    }
}
