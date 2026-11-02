use super::metadata::Metadata;
use crate::error::Result;
use serde_json::Value;
use std::collections::BTreeMap;
use xidl_parser::hir;

#[derive(Default)]
pub(super) struct TypeDocumentation {
    declarations: BTreeMap<String, Declaration>,
}

#[derive(Default)]
struct Declaration {
    metadata: Metadata,
    fields: BTreeMap<String, Metadata>,
    flattened: Vec<bool>,
}

impl TypeDocumentation {
    pub fn collect(&mut self, definitions: &[hir::Definition], scope: &[String]) {
        for definition in definitions {
            match definition {
                hir::Definition::ModuleDcl(module) => {
                    let mut path = scope.to_vec();
                    path.push(module.ident.clone());
                    self.collect(&module.definition, &path);
                }
                hir::Definition::ConstrTypeDcl(ty) => self.constructed(ty, scope),
                hir::Definition::TypeDcl(ty) => self.declaration(ty, scope),
                hir::Definition::ExceptDcl(ty) => {
                    self.members(&ty.ident, &ty.member, (&ty.annotations, scope))
                }
                hir::Definition::InterfaceDcl(interface) => {
                    if let hir::InterfaceDclInner::InterfaceDef(definition) = &interface.decl {
                        let mut path = scope.to_vec();
                        path.push(definition.header.ident.clone());
                        for export in definition.interface_body.iter().flat_map(|body| &body.0) {
                            match export {
                                hir::Export::TypeDcl(ty) => self.declaration(ty, &path),
                                hir::Export::ExceptDcl(ty) => {
                                    self.members(&ty.ident, &ty.member, (&ty.annotations, &path))
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn declaration(&mut self, ty: &hir::TypeDcl, scope: &[String]) {
        match ty {
            hir::TypeDcl::ConstrTypeDcl(ty) => self.constructed(ty, scope),
            hir::TypeDcl::TypedefDcl(ty) => {
                if let hir::TypedefType::ConstrTypeDcl(ty) = &ty.ty {
                    self.constructed(ty, scope);
                }
                for declarator in &ty.decl {
                    let (name, array_depth) = Metadata::declarator(declarator);
                    let declaration = self.entry(name, &ty.annotations, scope);
                    declaration.metadata.array_depth = array_depth;
                    if let hir::TypedefType::TypeSpec(ty) = &ty.ty {
                        declaration.metadata.ty = Some(ty.clone());
                    }
                }
            }
            hir::TypeDcl::NativeDcl(_) => {}
        }
    }

    fn constructed(&mut self, ty: &hir::ConstrTypeDcl, scope: &[String]) {
        match ty {
            hir::ConstrTypeDcl::StructDcl(ty) => {
                self.members(&ty.ident, &ty.member, (&ty.annotations, scope))
            }
            hir::ConstrTypeDcl::EnumDcl(ty) => {
                self.entry(&ty.ident, &ty.annotations, scope);
            }
            hir::ConstrTypeDcl::UnionDef(ty) => {
                let declaration = self.entry(&ty.ident, &ty.annotations, scope);
                for case in &ty.case {
                    let (name, array_depth) = Metadata::declarator(&case.element.value);
                    declaration.fields.insert(
                        name.into(),
                        Metadata {
                            annotations: case.element.annotations.clone(),
                            array_depth,
                            ty: match &case.element.ty {
                                hir::ElementSpecTy::TypeSpec(ty) => Some(ty.clone()),
                                _ => None,
                            },
                            ..Metadata::default()
                        },
                    );
                }
            }
            hir::ConstrTypeDcl::BitsetDcl(ty) => {
                self.entry(&ty.ident, &ty.annotations, scope);
            }
            hir::ConstrTypeDcl::BitmaskDcl(ty) => {
                self.entry(&ty.ident, &ty.annotations, scope);
            }
            _ => {}
        }
    }

    fn entry(
        &mut self,
        name: &str,
        annotations: &[hir::Annotation],
        scope: &[String],
    ) -> &mut Declaration {
        let mut path = scope.to_vec();
        path.push(name.into());
        let entry = self.declarations.entry(path.join(".")).or_default();
        entry.metadata.annotations = annotations.to_vec();
        entry
    }

    fn members(
        &mut self,
        name: &str,
        members: &[hir::Member],
        context: (&[hir::Annotation], &[String]),
    ) {
        let (annotations, scope) = context;
        let declaration = self.entry(name, annotations, scope);
        for member in members {
            if hir::is_skipped(&member.annotations) {
                continue;
            }
            if member
                .annotations
                .iter()
                .any(|annotation| hir::annotation_name(annotation) == Some("flatten"))
            {
                declaration.flattened.push(member.is_optional());
            }
            for declarator in &member.ident {
                let (name, array_depth) = Metadata::declarator(declarator);
                declaration.fields.insert(
                    hir::effective_wire_name(name, &member.annotations, annotations),
                    Metadata {
                        annotations: member.annotations.clone(),
                        ty: Some(member.ty.clone()),
                        declared_default: member.default_value(),
                        array_depth,
                    },
                );
            }
        }
    }

    pub fn enrich(&self, schemas: &mut Value) -> Result<()> {
        for (name, schema) in schemas.as_object_mut().into_iter().flatten() {
            if let Some(declaration) = self.declarations.get(name) {
                declaration.metadata.apply(schema)?;
                declaration.fields(schema)?;
            }
        }
        Ok(())
    }
}

impl Declaration {
    fn fields(&self, schema: &mut Value) -> Result<()> {
        for (name, field) in schema
            .get_mut("properties")
            .and_then(Value::as_object_mut)
            .into_iter()
            .flatten()
        {
            if let Some(metadata) = self.fields.get(name) {
                metadata.apply(field)?;
            }
        }
        for keyword in ["allOf", "oneOf"] {
            if let Some(parts) = schema.get_mut(keyword).and_then(Value::as_array_mut) {
                if keyword == "allOf" && parts.first().is_some_and(|part| part["type"] == "object")
                {
                    for (part, optional) in parts.iter_mut().skip(1).zip(&self.flattened) {
                        if *optional {
                            part["x-xidl-optional-flatten"] = true.into();
                        }
                    }
                }
                for part in parts {
                    self.fields(part)?;
                }
            }
        }
        Ok(())
    }
}
