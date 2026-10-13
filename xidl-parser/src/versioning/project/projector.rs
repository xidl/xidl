//! The projection walker: one version in, one ordinary specification out.

use super::super::VersioningError;
use super::super::lifecycle::{Availability, Site, SiteKind, availability};
use super::super::paths::{declarators_name, join};
use super::super::timeline::{VersionDef, VersionId, VersionTimeline};
use super::references::touch_const_type;
use super::references::touch_type;
use super::strip_annotations;
use crate::error::ParserResult;
use crate::hir::{
    Annotation, ConstDcl, Definition, ExceptDcl, ModuleDcl, TypeDcl, TypeSpec, TypedefDcl,
    TypedefType,
};
use std::collections::BTreeSet;

pub(super) struct Projector<'a> {
    pub(super) timeline: &'a VersionTimeline,
    pub(super) version: VersionId,
    pub(super) def: &'a VersionDef,
    kept: BTreeSet<String>,
    dropped: BTreeSet<String>,
    pub(super) references: Vec<(String, String)>,
}

impl<'a> Projector<'a> {
    pub(super) fn new(
        timeline: &'a VersionTimeline,
        version: VersionId,
        def: &'a VersionDef,
    ) -> Self {
        Self {
            timeline,
            version,
            def,
            kept: BTreeSet::new(),
            dropped: BTreeSet::new(),
            references: Vec::new(),
        }
    }

    /// Reports references to declarations that this projection drops.
    pub(super) fn check_references(&self) -> ParserResult<()> {
        for (decl, reference) in &self.references {
            if self.dropped.contains(reference) && !self.kept.contains(reference) {
                return Err(VersioningError::DanglingReference {
                    decl: decl.clone(),
                    reference: reference.clone(),
                    version: self.def.ident.clone(),
                }
                .into());
            }
        }
        Ok(())
    }

    pub(super) fn note_declaration(&mut self, ident: &str, is_kept: bool) {
        if ident.is_empty() {
            return;
        }
        if is_kept {
            self.kept.insert(ident.to_string());
        } else {
            self.dropped.insert(ident.to_string());
        }
    }

    pub(super) fn site(
        &self,
        path: &str,
        parent_path: &str,
        kind: SiteKind,
        annotations: &'a [Annotation],
    ) -> Site<'a> {
        Site {
            path: path.to_string(),
            parent_path: parent_path.to_string(),
            kind,
            annotations,
            enum_members: None,
        }
    }

    pub(super) fn definition(
        &mut self,
        definition: &Definition,
        parent: Availability,
        path: &str,
    ) -> ParserResult<Option<Definition>> {
        match definition {
            Definition::Pragma(pragma) => Ok(Some(Definition::Pragma(pragma.clone()))),
            Definition::ModuleDcl(module) => self.module(module, parent, path),
            Definition::TypeDcl(type_dcl) => Ok(self
                .type_dcl(type_dcl, parent, path)?
                .map(Definition::TypeDcl)),
            Definition::ConstrTypeDcl(constr) => Ok(self
                .constr(constr, parent, path)?
                .map(Definition::ConstrTypeDcl)),
            Definition::ConstDcl(const_dcl) => Ok(self
                .const_dcl(const_dcl, parent, path)?
                .map(Definition::ConstDcl)),
            Definition::ExceptDcl(except) => Ok(self
                .except_dcl(except, parent, path)?
                .map(Definition::ExceptDcl)),
            Definition::InterfaceDcl(interface) => Ok(self
                .interface_dcl(interface, parent, path)?
                .map(Definition::InterfaceDcl)),
        }
    }

    fn module(
        &mut self,
        module: &ModuleDcl,
        parent: Availability,
        path: &str,
    ) -> ParserResult<Option<Definition>> {
        let here = join(path, &module.ident);
        let site = self.site(&here, path, SiteKind::Module, &module.annotations);
        let own = availability(&site, parent, self.timeline)?;
        if !own.contains(self.version) {
            return Ok(None);
        }
        let mut definitions = Vec::new();
        for definition in &module.definition {
            if let Some(projected) = self.definition(definition, own, &here)? {
                definitions.push(projected);
            }
        }
        Ok(Some(Definition::ModuleDcl(ModuleDcl {
            annotations: strip_annotations(&module.annotations),
            ident: module.ident.clone(),
            definition: definitions,
        })))
    }

    pub(super) fn type_dcl(
        &mut self,
        type_dcl: &TypeDcl,
        parent: Availability,
        path: &str,
    ) -> ParserResult<Option<TypeDcl>> {
        match type_dcl {
            TypeDcl::ConstrTypeDcl(constr) => Ok(self
                .constr(constr, parent, path)?
                .map(TypeDcl::ConstrTypeDcl)),
            TypeDcl::TypedefDcl(typedef) => self.typedef_dcl(typedef, parent, path),
            TypeDcl::NativeDcl(native) => {
                let here = join(path, &native.decl.0);
                let site = self.site(&here, path, SiteKind::Type, &native.annotations);
                let own = availability(&site, parent, self.timeline)?;
                self.note_declaration(&native.decl.0, own.contains(self.version));
                if !own.contains(self.version) {
                    return Ok(None);
                }
                let mut projected = native.clone();
                projected.annotations = strip_annotations(&native.annotations);
                Ok(Some(TypeDcl::NativeDcl(projected)))
            }
        }
    }

    fn typedef_dcl(
        &mut self,
        typedef: &TypedefDcl,
        parent: Availability,
        path: &str,
    ) -> ParserResult<Option<TypeDcl>> {
        let names = declarators_name(&typedef.decl);
        let here = join(path, &names);
        let site = self.site(&here, path, SiteKind::Type, &typedef.annotations);
        let own = availability(&site, parent, self.timeline)?;
        for name in names.split(", ") {
            self.note_declaration(name, own.contains(self.version));
        }
        if !own.contains(self.version) {
            return Ok(None);
        }
        let mut projected = typedef.clone();
        projected.annotations = strip_annotations(&typedef.annotations);
        match &typedef.ty {
            TypedefType::TypeSpec(ty) => {
                let mut ty = ty.clone();
                self.touch_type(&mut ty, &here);
                projected.ty = TypedefType::TypeSpec(ty);
            }
            TypedefType::ConstrTypeDcl(constr) => {
                projected.ty = match self.constr(constr, own, &here)? {
                    Some(nested) => TypedefType::ConstrTypeDcl(nested),
                    None => return Ok(None),
                };
            }
        }
        Ok(Some(TypeDcl::TypedefDcl(projected)))
    }

    pub(super) fn const_dcl(
        &mut self,
        const_dcl: &ConstDcl,
        parent: Availability,
        path: &str,
    ) -> ParserResult<Option<ConstDcl>> {
        let here = join(path, &const_dcl.ident);
        let site = self.site(&here, path, SiteKind::Unsupported, &const_dcl.annotations);
        let own = availability(&site, parent, self.timeline)?;
        self.note_declaration(&const_dcl.ident, own.contains(self.version));
        if !own.contains(self.version) {
            return Ok(None);
        }
        let mut projected = const_dcl.clone();
        projected.annotations = strip_annotations(&const_dcl.annotations);
        touch_const_type(&mut projected.ty, &mut self.references, &here);
        Ok(Some(projected))
    }

    pub(super) fn except_dcl(
        &mut self,
        except: &ExceptDcl,
        parent: Availability,
        path: &str,
    ) -> ParserResult<Option<ExceptDcl>> {
        let here = join(path, &except.ident);
        let site = self.site(&here, path, SiteKind::Type, &except.annotations);
        let own = availability(&site, parent, self.timeline)?;
        self.note_declaration(&except.ident, own.contains(self.version));
        if !own.contains(self.version) {
            return Ok(None);
        }
        let mut projected = except.clone();
        projected.annotations = strip_annotations(&except.annotations);
        projected.member = self.members(&except.member, own, &here)?;
        Ok(Some(projected))
    }

    pub(super) fn touch_type(&mut self, ty: &mut TypeSpec, decl: &str) {
        touch_type(ty, &mut self.references, decl);
    }
}
