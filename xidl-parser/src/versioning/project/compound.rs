//! Projection of structs, unions, enums, bitmasks and forward declarations.

use super::super::lifecycle::{Availability, SiteKind, availability};
use super::super::paths::{declarator_name, join};
use super::projector::Projector;
use super::references::{
    touch_const_expr, touch_declarators, touch_scoped, touch_switch_type, touch_type,
};
use super::strip_annotations;
use crate::error::ParserResult;
use crate::hir::{
    Annotation, ConstrTypeDcl, ElementSpec, ElementSpecTy, StructDcl, StructForwardDcl, UnionDef,
    UnionForwardDcl,
};

impl<'a> Projector<'a> {
    pub(super) fn constr(
        &mut self,
        constr: &ConstrTypeDcl,
        parent: Availability,
        path: &str,
    ) -> ParserResult<Option<ConstrTypeDcl>> {
        match constr {
            ConstrTypeDcl::StructDcl(value) => self.struct_dcl(value, parent, path),
            ConstrTypeDcl::StructForwardDcl(value) => Ok(self
                .forward_declaration(&value.ident, &value.annotations, parent, path)?
                .map(|annotations| {
                    ConstrTypeDcl::StructForwardDcl(StructForwardDcl {
                        annotations,
                        ident: value.ident.clone(),
                    })
                })),
            ConstrTypeDcl::EnumDcl(value) => self.enum_dcl(value, parent, path),
            ConstrTypeDcl::UnionDef(value) => self.union_def(value, parent, path),
            ConstrTypeDcl::UnionForwardDcl(value) => Ok(self
                .forward_declaration(&value.ident, &value.annotations, parent, path)?
                .map(|annotations| {
                    ConstrTypeDcl::UnionForwardDcl(UnionForwardDcl {
                        annotations,
                        ident: value.ident.clone(),
                    })
                })),
            ConstrTypeDcl::BitsetDcl(value) => {
                let here = join(path, &value.ident);
                let site = self.site(&here, path, SiteKind::Type, &value.annotations);
                let own = availability(&site, parent, self.timeline)?;
                self.note_declaration(&here, own.contains(self.version));
                if !own.contains(self.version) {
                    return Ok(None);
                }
                let mut projected = value.clone();
                projected.annotations = strip_annotations(&value.annotations);
                if let Some(parent_name) = &mut projected.parent {
                    touch_scoped(parent_name, &mut self.references, &here);
                }
                Ok(Some(ConstrTypeDcl::BitsetDcl(projected)))
            }
            ConstrTypeDcl::BitmaskDcl(value) => {
                let here = join(path, &value.ident);
                let site = self.site(&here, path, SiteKind::Type, &value.annotations);
                let own = availability(&site, parent, self.timeline)?;
                self.note_declaration(&here, own.contains(self.version));
                if !own.contains(self.version) {
                    return Ok(None);
                }
                let mut values = Vec::new();
                for bit in &value.value {
                    let bit_path = join(&here, &bit.ident);
                    let site =
                        self.site(&bit_path, &here, SiteKind::BitmaskValue, &bit.annotations);
                    let bit_own = availability(&site, own, self.timeline)?;
                    if !bit_own.contains(self.version) {
                        continue;
                    }
                    let mut projected = bit.clone();
                    projected.annotations = strip_annotations(&bit.annotations);
                    values.push(projected);
                }
                let mut projected = value.clone();
                projected.annotations = strip_annotations(&value.annotations);
                projected.value = values;
                Ok(Some(ConstrTypeDcl::BitmaskDcl(projected)))
            }
        }
    }

    pub(super) fn struct_dcl(
        &mut self,
        value: &StructDcl,
        parent: Availability,
        path: &str,
    ) -> ParserResult<Option<ConstrTypeDcl>> {
        let here = join(path, &value.ident);
        let site = self.site(&here, path, SiteKind::Type, &value.annotations);
        let own = availability(&site, parent, self.timeline)?;
        self.note_declaration(&here, own.contains(self.version));
        if !own.contains(self.version) {
            return Ok(None);
        }
        let mut projected = value.clone();
        projected.annotations = strip_annotations(&value.annotations);
        for parent_name in &mut projected.parent {
            touch_scoped(parent_name, &mut self.references, &here);
        }
        projected.member = self.members(&value.member, own, &here)?;
        // Semantic analysis recomputes inherited members from the projected
        // base struct, so version availability filters them like own members.
        projected.inherited.clear();
        Ok(Some(ConstrTypeDcl::StructDcl(projected)))
    }

    pub(super) fn enum_dcl(
        &mut self,
        value: &crate::hir::EnumDcl,
        parent: Availability,
        path: &str,
    ) -> ParserResult<Option<ConstrTypeDcl>> {
        if value
            .annotations
            .iter()
            .any(|annotation| matches!(annotation, Annotation::Version))
        {
            // The version enum itself never appears in a projection, so
            // references to it must be reported instead of silently kept.
            let here = join(path, &value.ident);
            self.note_declaration(&here, false);
            for member in &value.member {
                self.note_declaration(&join(&here, &member.ident), false);
            }
            return Ok(None);
        }
        let here = join(path, &value.ident);
        let site = self.site(&here, path, SiteKind::Type, &value.annotations);
        let own = availability(&site, parent, self.timeline)?;
        self.note_declaration(&here, own.contains(self.version));
        if !own.contains(self.version) {
            return Ok(None);
        }
        let mut members = Vec::new();
        for member in &value.member {
            let member_path = join(&here, &member.ident);
            let site = self.site(
                &member_path,
                &here,
                SiteKind::Enumerator,
                &member.annotations,
            );
            let member_own = availability(&site, own, self.timeline)?;
            self.note_declaration(&member_path, member_own.contains(self.version));
            if !member_own.contains(self.version) {
                continue;
            }
            let mut member = member.clone();
            member.annotations = strip_annotations(&member.annotations);
            members.push(member);
        }
        let mut projected = value.clone();
        projected.annotations = strip_annotations(&value.annotations);
        projected.member = members;
        Ok(Some(ConstrTypeDcl::EnumDcl(projected)))
    }

    pub(super) fn union_def(
        &mut self,
        value: &UnionDef,
        parent: Availability,
        path: &str,
    ) -> ParserResult<Option<ConstrTypeDcl>> {
        let here = join(path, &value.ident);
        let site = self.site(&here, path, SiteKind::Type, &value.annotations);
        let own = availability(&site, parent, self.timeline)?;
        self.note_declaration(&here, own.contains(self.version));
        if !own.contains(self.version) {
            return Ok(None);
        }
        let mut switch = value.switch_type_spec.clone();
        touch_switch_type(&mut switch, &mut self.references, &here);
        let mut cases = Vec::new();
        for case in &value.case {
            let case_path = join(&here, &declarator_name(&case.element.value));
            let site = self.site(
                &case_path,
                &here,
                SiteKind::Member,
                &case.element.annotations,
            );
            let case_own = availability(&site, own, self.timeline)?;
            if !case_own.contains(self.version) {
                continue;
            }
            let mut labels = case.label.clone();
            for label in &mut labels {
                if let crate::hir::CaseLabel::Value(expr) = label {
                    touch_const_expr(expr, &mut self.references, &case_path);
                }
            }
            let mut element = ElementSpec {
                annotations: strip_annotations(&case.element.annotations),
                ty: case.element.ty.clone(),
                value: case.element.value.clone(),
                field_id: case.element.field_id,
                recursive: case.element.recursive,
            };
            touch_declarators(
                std::slice::from_mut(&mut element.value),
                &mut self.references,
                &case_path,
            );
            match &mut element.ty {
                ElementSpecTy::TypeSpec(ty) => touch_type(ty, &mut self.references, &case_path),
                ElementSpecTy::ConstrTypeDcl(constr) => {
                    element.ty = match self.constr(constr, case_own, &case_path)? {
                        Some(nested) => ElementSpecTy::ConstrTypeDcl(nested),
                        None => continue,
                    };
                }
            }
            let mut projected = case.clone();
            projected.label = labels;
            projected.element = element;
            cases.push(projected);
        }
        let mut projected = value.clone();
        projected.annotations = strip_annotations(&value.annotations);
        projected.switch_type_spec = switch;
        projected.case = cases;
        Ok(Some(ConstrTypeDcl::UnionDef(projected)))
    }

    /// Validates a forward declaration and returns its stripped annotations.
    pub(super) fn forward_declaration(
        &mut self,
        ident: &str,
        annotations: &[Annotation],
        parent: Availability,
        path: &str,
    ) -> ParserResult<Option<Vec<Annotation>>> {
        let here = join(path, ident);
        let site = self.site(&here, path, SiteKind::Type, annotations);
        let own = availability(&site, parent, self.timeline)?;
        self.note_declaration(&here, own.contains(self.version));
        if !own.contains(self.version) {
            return Ok(None);
        }
        Ok(Some(strip_annotations(annotations)))
    }
}
