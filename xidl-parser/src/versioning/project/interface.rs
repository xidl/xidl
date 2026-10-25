//! Projection of interfaces, their exports, operations and parameters.

use super::super::lifecycle::{Availability, SiteKind, availability};
use super::super::paths::{attribute_name, declarators_name, join};
use super::projector::Projector;
use super::strip_annotations;
use crate::error::ParserResult;
use crate::hir::{
    AttrDcl, AttrDclInner, Export, InterfaceDcl, InterfaceDclInner, Member, OpDcl, OpTypeSpec,
    ParamDcl,
};

impl<'a> Projector<'a> {
    pub(super) fn interface_dcl(
        &mut self,
        interface: &InterfaceDcl,
        parent: Availability,
        path: &str,
    ) -> ParserResult<Option<InterfaceDcl>> {
        let ident = match &interface.decl {
            InterfaceDclInner::InterfaceForwardDcl(forward) => forward.ident.clone(),
            InterfaceDclInner::InterfaceDef(def) => def.header.ident.clone(),
        };
        let here = join(path, &ident);
        let site = self.site(&here, path, SiteKind::Interface, &interface.annotations);
        let own = availability(&site, parent, self.timeline)?;
        self.note_declaration(&here, own.contains(self.version));
        if !own.contains(self.version) {
            return Ok(None);
        }
        let mut projected = interface.clone();
        projected.annotations = strip_annotations(&interface.annotations);
        if let InterfaceDclInner::InterfaceDef(def) = &mut projected.decl {
            if let Some(parent) = &mut def.header.parent {
                for crate::hir::InterfaceName(name) in &mut parent.0 {
                    self.touch_scoped(name, &here);
                }
            }
            if let Some(body) = &mut def.interface_body {
                let mut exports = Vec::new();
                for export in &body.0 {
                    if let Some(export) = self.export(export, own, &here)? {
                        exports.push(export);
                    }
                }
                body.0 = exports;
            }
        }
        Ok(Some(projected))
    }

    pub(super) fn export(
        &mut self,
        export: &Export,
        parent: Availability,
        path: &str,
    ) -> ParserResult<Option<Export>> {
        match export {
            Export::OpDcl(operation) => {
                Ok(self.operation(operation, parent, path)?.map(Export::OpDcl))
            }
            Export::AttrDcl(attribute) => Ok(self
                .attribute(attribute, parent, path)?
                .map(Export::AttrDcl)),
            Export::TypeDcl(type_dcl) => {
                Ok(self.type_dcl(type_dcl, parent, path)?.map(Export::TypeDcl))
            }
            Export::ConstDcl(const_dcl) => Ok(self
                .const_dcl(const_dcl, parent, path)?
                .map(Export::ConstDcl)),
            Export::ExceptDcl(except) => Ok(self
                .except_dcl(except, parent, path)?
                .map(Export::ExceptDcl)),
        }
    }

    pub(super) fn operation(
        &mut self,
        operation: &OpDcl,
        parent: Availability,
        path: &str,
    ) -> ParserResult<Option<OpDcl>> {
        let here = join(path, &operation.ident);
        let site = self.site(&here, path, SiteKind::Operation, &operation.annotations);
        let own = availability(&site, parent, self.timeline)?;
        if !own.contains(self.version) {
            return Ok(None);
        }
        let mut projected = operation.clone();
        projected.annotations = strip_annotations(&operation.annotations);
        if let OpTypeSpec::TypeSpec(ty) = &mut projected.ty {
            self.touch_type(ty, &here);
        }
        if let Some(parameters) = &operation.parameter {
            let mut kept = Vec::new();
            for parameter in &parameters.0 {
                if let Some(parameter) = self.parameter(parameter, own, &here)? {
                    kept.push(parameter);
                }
            }
            projected.parameter = Some(crate::hir::ParameterDcls(kept));
        }
        Ok(Some(projected))
    }

    pub(super) fn parameter(
        &mut self,
        parameter: &ParamDcl,
        parent: Availability,
        path: &str,
    ) -> ParserResult<Option<ParamDcl>> {
        let here = join(path, &parameter.declarator.0);
        let site = self.site(&here, path, SiteKind::Parameter, &parameter.annotations);
        let own = availability(&site, parent, self.timeline)?;
        if !own.contains(self.version) {
            return Ok(None);
        }
        let mut projected = parameter.clone();
        projected.annotations = strip_annotations(&parameter.annotations);
        self.touch_type(&mut projected.ty, &here);
        Ok(Some(projected))
    }

    pub(super) fn attribute(
        &mut self,
        attribute: &AttrDcl,
        parent: Availability,
        path: &str,
    ) -> ParserResult<Option<AttrDcl>> {
        let here = join(path, &attribute_name(attribute));
        let site = self.site(&here, path, SiteKind::Attribute, &attribute.annotations);
        let own = availability(&site, parent, self.timeline)?;
        if !own.contains(self.version) {
            return Ok(None);
        }
        let mut projected = attribute.clone();
        projected.annotations = strip_annotations(&attribute.annotations);
        match &mut projected.decl {
            AttrDclInner::ReadonlyAttrSpec(spec) => self.touch_type(&mut spec.ty, &here),
            AttrDclInner::AttrSpec(spec) => self.touch_type(&mut spec.ty, &here),
        }
        Ok(Some(projected))
    }

    pub(super) fn members(
        &mut self,
        members: &[Member],
        parent: Availability,
        path: &str,
    ) -> ParserResult<Vec<Member>> {
        let mut kept = Vec::new();
        for member in members {
            let here = join(path, &declarators_name(&member.ident));
            let site = self.site(&here, path, SiteKind::Member, &member.annotations);
            let own = availability(&site, parent, self.timeline)?;
            if !own.contains(self.version) {
                continue;
            }
            let mut projected = member.clone();
            projected.annotations = strip_annotations(&member.annotations);
            self.touch_type(&mut projected.ty, &here);
            self.touch_declarators(&mut projected.ident, &here);
            if let Some(default) = &mut projected.default {
                self.touch_const_expr(&mut default.0, &here);
            }
            kept.push(projected);
        }
        Ok(kept)
    }
}
