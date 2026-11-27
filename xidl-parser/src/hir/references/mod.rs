//! Structural traversal of references in semantic declarations.

mod annotations;

#[cfg(test)]
mod tests;

use super::*;

/// Visits scoped references without treating declaration identifiers as uses.
pub trait VisitScopedNames {
    /// Applies a transformation to every scoped reference in this value.
    fn visit_scoped_names(&mut self, visit: &mut impl FnMut(&mut ScopedName));

    /// Includes scoped references in structured annotation arguments.
    fn visit_scoped_names_with_annotations(&mut self, visit: &mut impl FnMut(&mut ScopedName)) {
        self.visit_scoped_names(visit);
    }

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

    fn visit_scoped_names_with_annotations(&mut self, visit: &mut impl FnMut(&mut ScopedName)) {
        for value in self {
            value.visit_scoped_names_with_annotations(visit);
        }
    }
}

struct ReferenceVisitor<F> {
    visit: F,
    include_annotations: bool,
}

trait WalkScopedNames {
    fn walk_scoped_names(&mut self, visitor: &mut ReferenceVisitor<impl FnMut(&mut ScopedName)>);
}

impl<T: WalkScopedNames> VisitScopedNames for T {
    fn visit_scoped_names(&mut self, visit: &mut impl FnMut(&mut ScopedName)) {
        self.walk_scoped_names(&mut ReferenceVisitor {
            visit,
            include_annotations: false,
        });
    }

    fn visit_scoped_names_with_annotations(&mut self, visit: &mut impl FnMut(&mut ScopedName)) {
        self.walk_scoped_names(&mut ReferenceVisitor {
            visit,
            include_annotations: true,
        });
    }
}

impl<T: WalkScopedNames> WalkScopedNames for [T] {
    fn walk_scoped_names(&mut self, visitor: &mut ReferenceVisitor<impl FnMut(&mut ScopedName)>) {
        for value in self {
            value.walk_scoped_names(visitor);
        }
    }
}

impl WalkScopedNames for ScopedName {
    fn walk_scoped_names(&mut self, visitor: &mut ReferenceVisitor<impl FnMut(&mut ScopedName)>) {
        (visitor.visit)(self);
    }
}

impl WalkScopedNames for TypeSpec {
    fn walk_scoped_names(&mut self, visitor: &mut ReferenceVisitor<impl FnMut(&mut ScopedName)>) {
        match self {
            Self::ScopedName(name) => (visitor.visit)(name),
            Self::SequenceType(sequence) => {
                sequence.ty.walk_scoped_names(visitor);
                if let Some(bound) = &mut sequence.len {
                    bound.0.walk_scoped_names(visitor);
                }
            }
            Self::MapType(map) => {
                map.key.walk_scoped_names(visitor);
                map.value.walk_scoped_names(visitor);
                if let Some(bound) = &mut map.len {
                    bound.0.walk_scoped_names(visitor);
                }
            }
            Self::TemplateType(template) => template.args.walk_scoped_names(visitor),
            Self::StringType(string) => {
                if let Some(bound) = &mut string.bound {
                    bound.0.walk_scoped_names(visitor);
                }
            }
            Self::WideStringType(string) => {
                if let Some(bound) = &mut string.bound {
                    bound.0.walk_scoped_names(visitor);
                }
            }
            Self::FixedPtType(fixed) => {
                fixed.integer.0.walk_scoped_names(visitor);
                fixed.fraction.0.walk_scoped_names(visitor);
            }
            _ => {}
        }
    }
}

impl WalkScopedNames for ConstExpr {
    fn walk_scoped_names(&mut self, visitor: &mut ReferenceVisitor<impl FnMut(&mut ScopedName)>) {
        match self {
            Self::ScopedName(name) => (visitor.visit)(name),
            Self::UnaryExpr(_, value) => value.walk_scoped_names(visitor),
            Self::BinaryExpr(_, left, right) => {
                left.walk_scoped_names(visitor);
                right.walk_scoped_names(visitor);
            }
            Self::Literal(_) => {}
        }
    }
}

impl WalkScopedNames for Declarator {
    fn walk_scoped_names(&mut self, visitor: &mut ReferenceVisitor<impl FnMut(&mut ScopedName)>) {
        if let Self::ArrayDeclarator(array) = self {
            for bound in &mut array.len {
                bound.0.walk_scoped_names(visitor);
            }
        }
    }
}

impl WalkScopedNames for Member {
    fn walk_scoped_names(&mut self, visitor: &mut ReferenceVisitor<impl FnMut(&mut ScopedName)>) {
        visitor.annotations(&mut self.annotations);
        self.ty.walk_scoped_names(visitor);
        self.ident.walk_scoped_names(visitor);
        if let Some(default) = &mut self.default {
            default.0.walk_scoped_names(visitor);
        }
    }
}

impl WalkScopedNames for ConstType {
    fn walk_scoped_names(&mut self, visitor: &mut ReferenceVisitor<impl FnMut(&mut ScopedName)>) {
        match self {
            Self::ScopedName(name) => (visitor.visit)(name),
            Self::SequenceType(sequence) => {
                sequence.ty.walk_scoped_names(visitor);
                if let Some(bound) = &mut sequence.len {
                    bound.0.walk_scoped_names(visitor);
                }
            }
            Self::StringType(string) => {
                if let Some(bound) = &mut string.bound {
                    bound.0.walk_scoped_names(visitor);
                }
            }
            Self::WideStringType(string) => {
                if let Some(bound) = &mut string.bound {
                    bound.0.walk_scoped_names(visitor);
                }
            }
            _ => {}
        }
    }
}

impl WalkScopedNames for Definition {
    fn walk_scoped_names(&mut self, visitor: &mut ReferenceVisitor<impl FnMut(&mut ScopedName)>) {
        match self {
            Self::ModuleDcl(module) => {
                visitor.annotations(&mut module.annotations);
                module.definition.walk_scoped_names(visitor);
            }
            Self::TypeDcl(decl) => decl.walk_scoped_names(visitor),
            Self::ConstrTypeDcl(constr) => constr.walk_scoped_names(visitor),
            Self::ConstDcl(decl) => {
                visitor.annotations(&mut decl.annotations);
                decl.ty.walk_scoped_names(visitor);
                decl.value.walk_scoped_names(visitor);
            }
            Self::ExceptDcl(except) => {
                visitor.annotations(&mut except.annotations);
                except.member.walk_scoped_names(visitor);
            }
            Self::InterfaceDcl(interface) => {
                visitor.annotations(&mut interface.annotations);
                if let InterfaceDclInner::InterfaceDef(def) = &mut interface.decl {
                    if let Some(parent) = &mut def.header.parent {
                        for InterfaceName(name) in &mut parent.0 {
                            (visitor.visit)(name);
                        }
                    }
                    if let Some(body) = &mut def.interface_body {
                        body.0.walk_scoped_names(visitor);
                    }
                }
            }
            Self::Pragma(_) => {}
        }
    }
}

impl WalkScopedNames for TypeDcl {
    fn walk_scoped_names(&mut self, visitor: &mut ReferenceVisitor<impl FnMut(&mut ScopedName)>) {
        match self {
            Self::ConstrTypeDcl(constr) => constr.walk_scoped_names(visitor),
            Self::TypedefDcl(alias) => {
                visitor.annotations(&mut alias.annotations);
                match &mut alias.ty {
                    TypedefType::TypeSpec(ty) => ty.walk_scoped_names(visitor),
                    TypedefType::ConstrTypeDcl(constr) => constr.walk_scoped_names(visitor),
                }
                alias.decl.walk_scoped_names(visitor);
            }
            Self::NativeDcl(decl) => visitor.annotations(&mut decl.annotations),
        }
    }
}

impl WalkScopedNames for ConstrTypeDcl {
    fn walk_scoped_names(&mut self, visitor: &mut ReferenceVisitor<impl FnMut(&mut ScopedName)>) {
        match self {
            Self::StructDcl(value) => {
                visitor.annotations(&mut value.annotations);
                for parent in &mut value.parent {
                    (visitor.visit)(parent);
                }
                value.member.walk_scoped_names(visitor);
                value.inherited.walk_scoped_names(visitor);
            }
            Self::UnionDef(value) => {
                visitor.annotations(&mut value.annotations);
                if let SwitchTypeSpec::ScopedName(name) = &mut value.switch_type_spec {
                    (visitor.visit)(name);
                }
                for case in &mut value.case {
                    visitor.annotations(&mut case.element.annotations);
                    for label in &mut case.label {
                        if let CaseLabel::Value(expression) = label {
                            expression.walk_scoped_names(visitor);
                        }
                    }
                    match &mut case.element.ty {
                        ElementSpecTy::TypeSpec(ty) => ty.walk_scoped_names(visitor),
                        ElementSpecTy::ConstrTypeDcl(constr) => constr.walk_scoped_names(visitor),
                    }
                    case.element.value.walk_scoped_names(visitor);
                }
            }
            Self::BitsetDcl(value) => {
                visitor.annotations(&mut value.annotations);
                if let Some(parent) = &mut value.parent {
                    (visitor.visit)(parent);
                }
                for field in &mut value.field {
                    field.pos.0.walk_scoped_names(visitor);
                }
            }
            Self::StructForwardDcl(value) => visitor.annotations(&mut value.annotations),
            Self::UnionForwardDcl(value) => visitor.annotations(&mut value.annotations),
            Self::EnumDcl(value) => {
                visitor.annotations(&mut value.annotations);
                for member in &mut value.member {
                    visitor.annotations(&mut member.annotations);
                }
            }
            Self::BitmaskDcl(value) => {
                visitor.annotations(&mut value.annotations);
                for member in &mut value.value {
                    visitor.annotations(&mut member.annotations);
                }
            }
        }
    }
}

impl WalkScopedNames for Export {
    fn walk_scoped_names(&mut self, visitor: &mut ReferenceVisitor<impl FnMut(&mut ScopedName)>) {
        match self {
            Self::TypeDcl(decl) => decl.walk_scoped_names(visitor),
            Self::ConstDcl(decl) => {
                visitor.annotations(&mut decl.annotations);
                decl.ty.walk_scoped_names(visitor);
                decl.value.walk_scoped_names(visitor);
            }
            Self::ExceptDcl(except) => {
                visitor.annotations(&mut except.annotations);
                except.member.walk_scoped_names(visitor);
            }
            Self::OpDcl(operation) => {
                visitor.annotations(&mut operation.annotations);
                if let OpTypeSpec::TypeSpec(ty) = &mut operation.ty {
                    ty.walk_scoped_names(visitor);
                }
                if let Some(parameters) = &mut operation.parameter {
                    for parameter in &mut parameters.0 {
                        visitor.annotations(&mut parameter.annotations);
                        parameter.ty.walk_scoped_names(visitor);
                    }
                }
                if let Some(raises) = &mut operation.raises {
                    for name in &mut raises.0 {
                        (visitor.visit)(name);
                    }
                }
            }
            Self::AttrDcl(attribute) => {
                visitor.annotations(&mut attribute.annotations);
                match &mut attribute.decl {
                    AttrDclInner::ReadonlyAttrSpec(spec) => {
                        spec.ty.walk_scoped_names(visitor);
                        if let Some(raises) = spec.declarator.raises_mut() {
                            for name in &mut raises.0 {
                                (visitor.visit)(name);
                            }
                        }
                    }
                    AttrDclInner::AttrSpec(spec) => {
                        spec.ty.walk_scoped_names(visitor);
                        if let AttrDeclarator::WithRaises { raises, .. } = &mut spec.declarator {
                            match raises {
                                AttrRaisesExpr::Case1(get, set) => {
                                    for name in &mut get.expr.0 {
                                        (visitor.visit)(name);
                                    }
                                    if let Some(set) = set {
                                        for name in &mut set.expr.0 {
                                            (visitor.visit)(name);
                                        }
                                    }
                                }
                                AttrRaisesExpr::SetExcepExpr(set) => {
                                    for name in &mut set.expr.0 {
                                        (visitor.visit)(name);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
