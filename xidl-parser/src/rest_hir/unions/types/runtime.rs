use super::{DeclarationKind, TypeDeclarations, has_annotation};
use crate::error::{ParseError, ParserResult};
use crate::hir;

impl TypeDeclarations<'_> {
    /// Builtins have no runtime model: only HTTP discriminators and labels may use them.
    pub(in crate::rest_hir::unions) fn validate_runtime_references(
        &self,
        definitions: &[hir::Definition],
        scope: &[String],
    ) -> ParserResult<()> {
        for definition in definitions {
            match definition {
                hir::Definition::ModuleDcl(module) => {
                    let mut nested = scope.to_vec();
                    nested.push(module.ident.clone());
                    self.validate_runtime_references(&module.definition, &nested)?;
                }
                hir::Definition::TypeDcl(ty) => self.runtime_type_decl(ty, scope)?,
                hir::Definition::ConstrTypeDcl(ty) => self.runtime_constructed(ty, scope)?,
                hir::Definition::ConstDcl(value) => self.runtime_const(value, scope)?,
                hir::Definition::ExceptDcl(value) => self.runtime_members(&value.member, scope)?,
                hir::Definition::InterfaceDcl(value) => self.runtime_interface(value, scope)?,
                hir::Definition::Pragma(_) => {}
            }
        }
        Ok(())
    }

    fn runtime_name(&self, name: &hir::ScopedName, scope: &[String]) -> ParserResult<()> {
        let mut reference = name.clone();
        while !reference.name.is_empty() {
            if let Some(decl) = self.resolve(&reference, scope) {
                if matches!(decl.kind, DeclarationKind::BuiltinEnum(_)) {
                    return Err(ParseError::Message(format!(
                        "HTTP built-in '{}' is compile-time only; use it only as an @http union discriminator or case label (reference '{}')",
                        decl.ident,
                        name.name.join("::")
                    )));
                }
                break;
            }
            reference.name.pop();
        }
        Ok(())
    }

    fn runtime_expr(&self, expr: &hir::ConstExpr, scope: &[String]) -> ParserResult<()> {
        match expr {
            hir::ConstExpr::ScopedName(name) => self.runtime_name(name, scope)?,
            hir::ConstExpr::UnaryExpr(_, inner) => self.runtime_expr(inner, scope)?,
            hir::ConstExpr::BinaryExpr(_, left, right) => {
                self.runtime_expr(left, scope)?;
                self.runtime_expr(right, scope)?;
            }
            hir::ConstExpr::Literal(_) => {}
        }
        Ok(())
    }

    fn runtime_bound(
        &self,
        bound: &Option<hir::PositiveIntConst>,
        scope: &[String],
    ) -> ParserResult<()> {
        if let Some(bound) = bound {
            self.runtime_expr(&bound.0, scope)?;
        }
        Ok(())
    }

    fn runtime_type(&self, ty: &hir::TypeSpec, scope: &[String]) -> ParserResult<()> {
        match ty {
            hir::TypeSpec::ScopedName(name) => self.runtime_name(name, scope)?,
            hir::TypeSpec::SequenceType(sequence) => {
                self.runtime_type(&sequence.ty, scope)?;
                self.runtime_bound(&sequence.len, scope)?;
            }
            hir::TypeSpec::MapType(map) => {
                self.runtime_type(&map.key, scope)?;
                self.runtime_type(&map.value, scope)?;
                self.runtime_bound(&map.len, scope)?;
            }
            hir::TypeSpec::TemplateType(template) => {
                self.runtime_name(
                    &hir::ScopedName {
                        name: vec![template.ident.clone()],
                        is_root: false,
                    },
                    scope,
                )?;
                for arg in &template.args {
                    self.runtime_type(arg, scope)?;
                }
            }
            hir::TypeSpec::StringType(string) => self.runtime_bound(&string.bound, scope)?,
            hir::TypeSpec::WideStringType(string) => self.runtime_bound(&string.bound, scope)?,
            hir::TypeSpec::FixedPtType(fixed) => {
                self.runtime_expr(&fixed.integer.0, scope)?;
                self.runtime_expr(&fixed.fraction.0, scope)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn runtime_declarator(&self, decl: &hir::Declarator, scope: &[String]) -> ParserResult<()> {
        if let hir::Declarator::ArrayDeclarator(array) = decl {
            for length in &array.len {
                self.runtime_expr(&length.0, scope)?;
            }
        }
        Ok(())
    }

    fn runtime_members(&self, members: &[hir::Member], scope: &[String]) -> ParserResult<()> {
        for member in members {
            self.runtime_type(&member.ty, scope)?;
            for decl in &member.ident {
                self.runtime_declarator(decl, scope)?;
            }
            if let Some(default) = &member.default {
                self.runtime_expr(&default.0, scope)?;
            }
        }
        Ok(())
    }

    fn runtime_const(&self, value: &hir::ConstDcl, scope: &[String]) -> ParserResult<()> {
        match &value.ty {
            hir::ConstType::ScopedName(name) => self.runtime_name(name, scope)?,
            hir::ConstType::SequenceType(sequence) => {
                self.runtime_type(&sequence.ty, scope)?;
                self.runtime_bound(&sequence.len, scope)?;
            }
            hir::ConstType::StringType(string) => self.runtime_bound(&string.bound, scope)?,
            hir::ConstType::WideStringType(string) => self.runtime_bound(&string.bound, scope)?,
            _ => {}
        }
        self.runtime_expr(&value.value, scope)
    }

    fn runtime_type_decl(&self, ty: &hir::TypeDcl, scope: &[String]) -> ParserResult<()> {
        match ty {
            hir::TypeDcl::ConstrTypeDcl(ty) => self.runtime_constructed(ty, scope)?,
            hir::TypeDcl::TypedefDcl(alias) => {
                match &alias.ty {
                    hir::TypedefType::TypeSpec(ty) => self.runtime_type(ty, scope)?,
                    hir::TypedefType::ConstrTypeDcl(ty) => self.runtime_constructed(ty, scope)?,
                }
                for decl in &alias.decl {
                    self.runtime_declarator(decl, scope)?;
                }
            }
            hir::TypeDcl::NativeDcl(_) => {}
        }
        Ok(())
    }

    fn runtime_constructed(&self, ty: &hir::ConstrTypeDcl, scope: &[String]) -> ParserResult<()> {
        match ty {
            hir::ConstrTypeDcl::StructDcl(value) => {
                for parent in &value.parent {
                    self.runtime_name(parent, scope)?;
                }
                self.runtime_members(&value.member, scope)?;
            }
            hir::ConstrTypeDcl::UnionDef(union) => {
                let http = has_annotation(&union.annotations, "http");
                if !http && let hir::SwitchTypeSpec::ScopedName(name) = &union.switch_type_spec {
                    self.runtime_name(name, scope)?;
                }
                for case in &union.case {
                    if !http {
                        for label in &case.label {
                            if let hir::CaseLabel::Value(value) = label {
                                self.runtime_expr(value, scope)?;
                            }
                        }
                    }
                    match &case.element.ty {
                        hir::ElementSpecTy::TypeSpec(ty) => self.runtime_type(ty, scope)?,
                        hir::ElementSpecTy::ConstrTypeDcl(ty) => {
                            self.runtime_constructed(ty, scope)?
                        }
                    }
                    self.runtime_declarator(&case.element.value, scope)?;
                }
            }
            hir::ConstrTypeDcl::BitsetDcl(value) => {
                if let Some(parent) = &value.parent {
                    self.runtime_name(parent, scope)?;
                }
                for field in &value.field {
                    self.runtime_expr(&field.pos.0, scope)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn runtime_interface(
        &self,
        interface: &hir::InterfaceDcl,
        scope: &[String],
    ) -> ParserResult<()> {
        let hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
            return Ok(());
        };
        if let Some(parents) = &def.header.parent {
            for parent in &parents.0 {
                self.runtime_name(&parent.0, scope)?;
            }
        }
        let mut nested = scope.to_vec();
        nested.push(def.header.ident.clone());
        if let Some((_, union)) = self.http_unions().find(|(path, _)| *path == nested) {
            return Err(ParseError::Message(format!(
                "@http union '{}' is declared inside interface '{}'; declare it at module or root scope",
                union.ident, def.header.ident
            )));
        }
        if let Some(body) = &def.interface_body {
            for export in &body.0 {
                match export {
                    hir::Export::TypeDcl(ty) => self.runtime_type_decl(ty, &nested)?,
                    hir::Export::ConstDcl(value) => self.runtime_const(value, &nested)?,
                    hir::Export::ExceptDcl(value) => {
                        self.runtime_members(&value.member, &nested)?
                    }
                    hir::Export::OpDcl(op) => {
                        if let hir::OpTypeSpec::TypeSpec(ty) = &op.ty {
                            self.runtime_type(ty, &nested)?;
                        }
                        if let Some(params) = &op.parameter {
                            for param in &params.0 {
                                self.runtime_type(&param.ty, &nested)?;
                            }
                        }
                        if let Some(raises) = &op.raises {
                            for name in &raises.0 {
                                self.runtime_name(name, &nested)?;
                            }
                        }
                    }
                    hir::Export::AttrDcl(attr) => self.runtime_attr(attr, &nested)?,
                }
            }
        }
        Ok(())
    }

    fn runtime_attr(&self, attr: &hir::AttrDcl, scope: &[String]) -> ParserResult<()> {
        match &attr.decl {
            hir::AttrDclInner::ReadonlyAttrSpec(spec) => {
                self.runtime_type(&spec.ty, scope)?;
                if let hir::ReadonlyAttrDeclarator::RaisesExpr(raises) = &spec.declarator {
                    for name in &raises.0 {
                        self.runtime_name(name, scope)?;
                    }
                }
            }
            hir::AttrDclInner::AttrSpec(spec) => {
                self.runtime_type(&spec.ty, scope)?;
                if let hir::AttrDeclarator::WithRaises { raises, .. } = &spec.declarator {
                    match raises {
                        hir::AttrRaisesExpr::Case1(get, set) => {
                            for name in &get.expr.0 {
                                self.runtime_name(name, scope)?;
                            }
                            if let Some(set) = set {
                                for name in &set.expr.0 {
                                    self.runtime_name(name, scope)?;
                                }
                            }
                        }
                        hir::AttrRaisesExpr::SetExcepExpr(set) => {
                            for name in &set.expr.0 {
                                self.runtime_name(name, scope)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
