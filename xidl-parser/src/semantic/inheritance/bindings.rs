//! Keep inherited references bound to their declaring scope.

use crate::hir::{ConstExpr, DeclarationNames, Declarator, Member, ScopedName, TypeSpec};

/// References resolved where an ancestor declared its members.
pub(super) struct MemberBinding<'a> {
    names: &'a DeclarationNames,
    scope: Vec<String>,
}

impl<'a> MemberBinding<'a> {
    /// Anchors inherited references to the ancestor's declaration scope.
    pub(super) fn new(names: &'a DeclarationNames, scope: Vec<String>) -> Self {
        Self { names, scope }
    }

    /// Resolves known names while preserving aliases and external references.
    pub(super) fn bind(&self, member: &mut Member) {
        self.ty(&mut member.ty);
        for declarator in &mut member.ident {
            if let Declarator::ArrayDeclarator(array) = declarator {
                for bound in &mut array.len {
                    self.expression(&mut bound.0);
                }
            }
        }
        if let Some(default) = &mut member.default {
            self.expression(&mut default.0);
        }
    }

    fn name(&self, name: &mut ScopedName) {
        if let Some(resolved) = self.names.resolve(&self.scope, name) {
            *name = resolved;
        }
    }

    fn ty(&self, ty: &mut TypeSpec) {
        match ty {
            TypeSpec::ScopedName(name) => self.name(name),
            TypeSpec::SequenceType(sequence) => {
                self.ty(&mut sequence.ty);
                if let Some(bound) = &mut sequence.len {
                    self.expression(&mut bound.0);
                }
            }
            TypeSpec::MapType(map) => {
                self.ty(&mut map.key);
                self.ty(&mut map.value);
                if let Some(bound) = &mut map.len {
                    self.expression(&mut bound.0);
                }
            }
            TypeSpec::TemplateType(template) => {
                for argument in &mut template.args {
                    self.ty(argument);
                }
            }
            TypeSpec::StringType(string) => {
                if let Some(bound) = &mut string.bound {
                    self.expression(&mut bound.0);
                }
            }
            TypeSpec::WideStringType(string) => {
                if let Some(bound) = &mut string.bound {
                    self.expression(&mut bound.0);
                }
            }
            TypeSpec::FixedPtType(fixed) => {
                self.expression(&mut fixed.integer.0);
                self.expression(&mut fixed.fraction.0);
            }
            _ => {}
        }
    }

    fn expression(&self, expression: &mut ConstExpr) {
        match expression {
            ConstExpr::ScopedName(name) => self.name(name),
            ConstExpr::UnaryExpr(_, value) => self.expression(value),
            ConstExpr::BinaryExpr(_, left, right) => {
                self.expression(left);
                self.expression(right);
            }
            ConstExpr::Literal(_) => {}
        }
    }
}
