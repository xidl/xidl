//! Structural traversal of references, preserving declarations and annotations.

#[cfg(test)]
mod tests;

use super::*;

/// Visits scoped references without treating declaration identifiers as uses.
pub trait VisitScopedNames {
    /// Applies a transformation to every scoped reference in this value.
    fn visit_scoped_names(&mut self, visit: &mut impl FnMut(&mut ScopedName));

    /// Removes one enclosing root namespace while keeping names absolute within
    /// it. Relative names and references outside that namespace stay unchanged.
    fn remove_root_namespace(&mut self, namespace: &str) {
        self.visit_scoped_names(&mut |name| {
            if name.is_root && name.name.first().is_some_and(|part| part == namespace) {
                name.name.remove(0);
            }
        });
    }
}

impl<T: VisitScopedNames> VisitScopedNames for Vec<T> {
    fn visit_scoped_names(&mut self, visit: &mut impl FnMut(&mut ScopedName)) {
        for value in self {
            value.visit_scoped_names(visit);
        }
    }
}

impl VisitScopedNames for ScopedName {
    fn visit_scoped_names(&mut self, visit: &mut impl FnMut(&mut ScopedName)) {
        visit(self);
    }
}

impl VisitScopedNames for TypeSpec {
    fn visit_scoped_names(&mut self, visit: &mut impl FnMut(&mut ScopedName)) {
        match self {
            Self::ScopedName(name) => visit(name),
            Self::SequenceType(sequence) => {
                sequence.ty.visit_scoped_names(visit);
                if let Some(bound) = &mut sequence.len {
                    bound.0.visit_scoped_names(visit);
                }
            }
            Self::MapType(map) => {
                map.key.visit_scoped_names(visit);
                map.value.visit_scoped_names(visit);
                if let Some(bound) = &mut map.len {
                    bound.0.visit_scoped_names(visit);
                }
            }
            Self::TemplateType(template) => template.args.visit_scoped_names(visit),
            Self::StringType(string) => {
                if let Some(bound) = &mut string.bound {
                    bound.0.visit_scoped_names(visit);
                }
            }
            Self::WideStringType(string) => {
                if let Some(bound) = &mut string.bound {
                    bound.0.visit_scoped_names(visit);
                }
            }
            Self::FixedPtType(fixed) => {
                fixed.integer.0.visit_scoped_names(visit);
                fixed.fraction.0.visit_scoped_names(visit);
            }
            _ => {}
        }
    }
}

impl VisitScopedNames for ConstExpr {
    fn visit_scoped_names(&mut self, visit: &mut impl FnMut(&mut ScopedName)) {
        match self {
            Self::ScopedName(name) => visit(name),
            Self::UnaryExpr(_, value) => value.visit_scoped_names(visit),
            Self::BinaryExpr(_, left, right) => {
                left.visit_scoped_names(visit);
                right.visit_scoped_names(visit);
            }
            Self::Literal(_) => {}
        }
    }
}

impl VisitScopedNames for Declarator {
    fn visit_scoped_names(&mut self, visit: &mut impl FnMut(&mut ScopedName)) {
        if let Self::ArrayDeclarator(array) = self {
            for bound in &mut array.len {
                bound.0.visit_scoped_names(visit);
            }
        }
    }
}

impl VisitScopedNames for Member {
    fn visit_scoped_names(&mut self, visit: &mut impl FnMut(&mut ScopedName)) {
        self.ty.visit_scoped_names(visit);
        self.ident.visit_scoped_names(visit);
        if let Some(default) = &mut self.default {
            default.0.visit_scoped_names(visit);
        }
    }
}

impl VisitScopedNames for ConstType {
    fn visit_scoped_names(&mut self, visit: &mut impl FnMut(&mut ScopedName)) {
        match self {
            Self::ScopedName(name) => visit(name),
            Self::SequenceType(sequence) => {
                sequence.ty.visit_scoped_names(visit);
                if let Some(bound) = &mut sequence.len {
                    bound.0.visit_scoped_names(visit);
                }
            }
            Self::StringType(string) => {
                if let Some(bound) = &mut string.bound {
                    bound.0.visit_scoped_names(visit);
                }
            }
            Self::WideStringType(string) => {
                if let Some(bound) = &mut string.bound {
                    bound.0.visit_scoped_names(visit);
                }
            }
            _ => {}
        }
    }
}

impl VisitScopedNames for Definition {
    fn visit_scoped_names(&mut self, visit: &mut impl FnMut(&mut ScopedName)) {
        match self {
            Self::ModuleDcl(module) => module.definition.visit_scoped_names(visit),
            Self::TypeDcl(decl) => decl.visit_scoped_names(visit),
            Self::ConstrTypeDcl(constr) => constr.visit_scoped_names(visit),
            Self::ConstDcl(decl) => {
                decl.ty.visit_scoped_names(visit);
                decl.value.visit_scoped_names(visit);
            }
            Self::ExceptDcl(except) => except.member.visit_scoped_names(visit),
            Self::InterfaceDcl(interface) => {
                if let InterfaceDclInner::InterfaceDef(def) = &mut interface.decl {
                    if let Some(parent) = &mut def.header.parent {
                        for InterfaceName(name) in &mut parent.0 {
                            visit(name);
                        }
                    }
                    if let Some(body) = &mut def.interface_body {
                        body.0.visit_scoped_names(visit);
                    }
                }
            }
            Self::Pragma(_) => {}
        }
    }
}

impl VisitScopedNames for TypeDcl {
    fn visit_scoped_names(&mut self, visit: &mut impl FnMut(&mut ScopedName)) {
        match self {
            Self::ConstrTypeDcl(constr) => constr.visit_scoped_names(visit),
            Self::TypedefDcl(alias) => {
                match &mut alias.ty {
                    TypedefType::TypeSpec(ty) => ty.visit_scoped_names(visit),
                    TypedefType::ConstrTypeDcl(constr) => constr.visit_scoped_names(visit),
                }
                alias.decl.visit_scoped_names(visit);
            }
            Self::NativeDcl(_) => {}
        }
    }
}

impl VisitScopedNames for ConstrTypeDcl {
    fn visit_scoped_names(&mut self, visit: &mut impl FnMut(&mut ScopedName)) {
        match self {
            Self::StructDcl(value) => {
                for parent in &mut value.parent {
                    visit(parent);
                }
                value.member.visit_scoped_names(visit);
                value.inherited.visit_scoped_names(visit);
            }
            Self::UnionDef(value) => {
                if let SwitchTypeSpec::ScopedName(name) = &mut value.switch_type_spec {
                    visit(name);
                }
                for case in &mut value.case {
                    for label in &mut case.label {
                        if let CaseLabel::Value(expression) = label {
                            expression.visit_scoped_names(visit);
                        }
                    }
                    match &mut case.element.ty {
                        ElementSpecTy::TypeSpec(ty) => ty.visit_scoped_names(visit),
                        ElementSpecTy::ConstrTypeDcl(constr) => constr.visit_scoped_names(visit),
                    }
                    case.element.value.visit_scoped_names(visit);
                }
            }
            Self::BitsetDcl(value) => {
                if let Some(parent) = &mut value.parent {
                    visit(parent);
                }
                for field in &mut value.field {
                    field.pos.0.visit_scoped_names(visit);
                }
            }
            _ => {}
        }
    }
}

impl VisitScopedNames for Export {
    fn visit_scoped_names(&mut self, visit: &mut impl FnMut(&mut ScopedName)) {
        match self {
            Self::TypeDcl(decl) => decl.visit_scoped_names(visit),
            Self::ConstDcl(decl) => {
                decl.ty.visit_scoped_names(visit);
                decl.value.visit_scoped_names(visit);
            }
            Self::ExceptDcl(except) => except.member.visit_scoped_names(visit),
            Self::OpDcl(operation) => {
                if let OpTypeSpec::TypeSpec(ty) = &mut operation.ty {
                    ty.visit_scoped_names(visit);
                }
                if let Some(parameters) = &mut operation.parameter {
                    for parameter in &mut parameters.0 {
                        parameter.ty.visit_scoped_names(visit);
                    }
                }
                if let Some(raises) = &mut operation.raises {
                    for name in &mut raises.0 {
                        visit(name);
                    }
                }
            }
            Self::AttrDcl(attribute) => match &mut attribute.decl {
                AttrDclInner::ReadonlyAttrSpec(spec) => {
                    spec.ty.visit_scoped_names(visit);
                    if let Some(raises) = spec.declarator.raises_mut() {
                        for name in &mut raises.0 {
                            visit(name);
                        }
                    }
                }
                AttrDclInner::AttrSpec(spec) => {
                    spec.ty.visit_scoped_names(visit);
                    if let AttrDeclarator::WithRaises { raises, .. } = &mut spec.declarator {
                        match raises {
                            AttrRaisesExpr::Case1(get, set) => {
                                for name in &mut get.expr.0 {
                                    visit(name);
                                }
                                if let Some(set) = set {
                                    for name in &mut set.expr.0 {
                                        visit(name);
                                    }
                                }
                            }
                            AttrRaisesExpr::SetExcepExpr(set) => {
                                for name in &mut set.expr.0 {
                                    visit(name);
                                }
                            }
                        }
                    }
                }
            },
        }
    }
}
