//! The projection walker: one version in, one ordinary specification out.

use super::super::VersioningError;
use super::super::lifecycle::{Availability, Site, SiteKind, availability};
use super::super::paths::{declarators_name, join};
use super::super::timeline::{VersionDef, VersionId, VersionTimeline};
use super::references::Reference;
use super::strip_annotations;
use crate::error::ParserResult;
use crate::hir::DeclarationNames;
use crate::hir::{
    Annotation, AttrDclInner, AttrDeclarator, AttrRaisesExpr, ConstDcl, Definition, ExceptDcl,
    Export, InterfaceDclInner, ModuleDcl, ReadonlyAttrDeclarator, ScopedName, TypeDcl, TypedefDcl,
    TypedefType,
};
use std::collections::BTreeSet;

pub(super) struct Projector<'a> {
    pub(super) timeline: &'a VersionTimeline,
    pub(super) version: VersionId,
    pub(super) def: &'a VersionDef,
    kept_paths: BTreeSet<String>,
    dropped_paths: BTreeSet<String>,
    pub(super) references: Vec<Reference>,
    pub(super) declarations: DeclarationNames,
}

impl<'a> Projector<'a> {
    pub(super) fn new(
        timeline: &'a VersionTimeline,
        version: VersionId,
        def: &'a VersionDef,
        definitions: &[Definition],
    ) -> Self {
        Self {
            timeline,
            version,
            def,
            kept_paths: BTreeSet::new(),
            dropped_paths: BTreeSet::new(),
            references: Vec::new(),
            declarations: DeclarationNames::collect(definitions),
        }
    }

    /// Drops `raises(...)` entries whose exception this projection removed.
    ///
    /// An exception set has no annotation slot, so an operation can only raise
    /// a versioned exception in the versions that declare it.
    pub(super) fn prune_raises(&self, definitions: &mut [Definition]) {
        for definition in definitions {
            self.prune_definition(definition);
        }
    }

    fn prune_definition(&self, definition: &mut Definition) {
        match definition {
            Definition::ModuleDcl(module) => self.prune_raises(&mut module.definition),
            Definition::InterfaceDcl(interface) => {
                let InterfaceDclInner::InterfaceDef(def) = &mut interface.decl else {
                    return;
                };
                let Some(body) = &mut def.interface_body else {
                    return;
                };
                for export in &mut body.0 {
                    self.prune_export(export);
                }
            }
            _ => {}
        }
    }

    fn prune_export(&self, export: &mut Export) {
        match export {
            Export::OpDcl(operation) => {
                if let Some(raises) = operation.raises.as_mut() {
                    self.prune_names(&mut raises.0);
                    if raises.0.is_empty() {
                        operation.raises = None;
                    }
                }
            }
            Export::AttrDcl(attribute) => match &mut attribute.decl {
                AttrDclInner::ReadonlyAttrSpec(spec) => {
                    if let ReadonlyAttrDeclarator::WithRaises { declarator, raises } =
                        &mut spec.declarator
                    {
                        self.prune_names(&mut raises.0);
                        if raises.0.is_empty() {
                            spec.declarator =
                                ReadonlyAttrDeclarator::SimpleDeclarator(vec![declarator.clone()]);
                        }
                    }
                }
                AttrDclInner::AttrSpec(spec) => {
                    if let AttrDeclarator::WithRaises { declarator, raises } = &mut spec.declarator
                    {
                        match raises {
                            AttrRaisesExpr::Case1(get, set) => {
                                self.prune_names(&mut get.expr.0);
                                if let Some(set) = set {
                                    self.prune_names(&mut set.expr.0);
                                }
                                match (get.expr.0.is_empty(), set.as_ref()) {
                                    (true, Some(set)) if !set.expr.0.is_empty() => {
                                        *raises = AttrRaisesExpr::SetExcepExpr(set.clone());
                                    }
                                    (false, Some(set)) if set.expr.0.is_empty() => {
                                        *raises = AttrRaisesExpr::Case1(get.clone(), None);
                                    }
                                    (true, _) => {
                                        spec.declarator = AttrDeclarator::SimpleDeclarator(vec![
                                            declarator.clone(),
                                        ]);
                                    }
                                    _ => {}
                                }
                            }
                            AttrRaisesExpr::SetExcepExpr(set) => {
                                self.prune_names(&mut set.expr.0);
                                if set.expr.0.is_empty() {
                                    spec.declarator =
                                        AttrDeclarator::SimpleDeclarator(vec![declarator.clone()]);
                                }
                            }
                        }
                    }
                }
            },
            _ => {}
        }
    }

    fn prune_names(&self, names: &mut Vec<ScopedName>) {
        names.retain(|name| !self.reference_unavailable(&name.name, name.is_root));
        for name in names {
            self.qualify_root(name);
        }
    }

    /// Reports references to declarations that this projection drops.
    pub(super) fn check_references(&self) -> ParserResult<()> {
        for reference in &self.references {
            if self.reference_unavailable(&reference.segments, reference.is_root) {
                return Err(VersioningError::DanglingReference {
                    decl: reference.decl.clone(),
                    reference: reference.rendered(),
                    version: self.def.ident.clone(),
                }
                .into());
            }
        }
        Ok(())
    }

    /// Whether no surviving declaration can satisfy a reference.
    ///
    /// Paths are matched by suffix so a scoped reference such as `models::Item`
    /// is only reported when every declaration it could name is gone, which
    /// keeps same-named declarations in other modules from masking a dangling
    /// reference.
    fn reference_unavailable(&self, segments: &[String], is_root: bool) -> bool {
        if segments.is_empty() {
            return false;
        }
        let target = segments.join("::");
        let matches =
            |path: &String| path == &target || (!is_root && path.ends_with(&format!("::{target}")));
        if self.kept_paths.iter().any(matches) {
            return false;
        }
        self.dropped_paths.iter().any(matches)
    }

    pub(super) fn note_declaration(&mut self, path: &str, is_kept: bool) {
        if path.is_empty() {
            return;
        }
        if is_kept {
            self.kept_paths.insert(path.to_string());
        } else {
            self.dropped_paths.insert(path.to_string());
        }
    }

    /// Records every declaration inside a dropped subtree as dropped.
    pub(super) fn note_dropped_subtree(&mut self, definition: &Definition, path: &str) {
        let mut paths = Vec::new();
        DeclarationNames::visit(
            std::slice::from_ref(definition),
            path,
            &mut |declared: &str| {
                paths.push(declared.to_string());
            },
        );
        for declared in paths {
            self.note_declaration(&declared, false);
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
            parameter: None,
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
            self.note_dropped_subtree(&Definition::ModuleDcl(module.clone()), path);
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
                self.note_declaration(&here, own.contains(self.version));
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
        if !own.contains(self.version) {
            self.note_dropped_subtree(
                &Definition::TypeDcl(TypeDcl::TypedefDcl(typedef.clone())),
                path,
            );
            return Ok(None);
        }
        for declarator in &typedef.decl {
            self.note_declaration(
                &join(path, &super::super::paths::declarator_name(declarator)),
                true,
            );
        }
        let mut projected = typedef.clone();
        projected.annotations = strip_annotations(&typedef.annotations);
        self.touch_declarators(&mut projected.decl, &here);
        match &typedef.ty {
            TypedefType::TypeSpec(ty) => {
                let mut ty = ty.clone();
                self.touch_type(&mut ty, &here);
                projected.ty = TypedefType::TypeSpec(ty);
            }
            TypedefType::ConstrTypeDcl(constr) => {
                projected.ty = match self.constr(constr, own, path)? {
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
        self.note_declaration(&here, own.contains(self.version));
        if !own.contains(self.version) {
            return Ok(None);
        }
        let mut projected = const_dcl.clone();
        projected.annotations = strip_annotations(&const_dcl.annotations);
        self.touch_const_type(&mut projected.ty, &here);
        self.touch_const_expr(&mut projected.value, &here);
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
        self.note_declaration(&here, own.contains(self.version));
        if !own.contains(self.version) {
            return Ok(None);
        }
        let mut projected = except.clone();
        projected.annotations = strip_annotations(&except.annotations);
        projected.member = self.members(&except.member, own, &here)?;
        Ok(Some(projected))
    }
}
