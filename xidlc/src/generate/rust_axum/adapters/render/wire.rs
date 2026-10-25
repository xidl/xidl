//! Decode served-version wire layouts into their exact public model types.

use super::super::construction::{ConstructionField, ConstructionValue};
use super::*;
use crate::generate::adapter::wire_layout::WireLayouts;
use crate::generate::adapter::{TypeShape, TypeUse, VersionPlan};
use crate::generate::rust::util::render_const;
use crate::generate::rust::util::union_serde_tag_string;
use xidl_parser::hir;

struct WireBuilder<'a> {
    renderer: &'a AdapterRenderer<'a>,
    pending: BTreeSet<(String, String)>,
    visited: BTreeSet<(String, String)>,
}

impl AdapterRenderer<'_> {
    pub(super) fn wire(&self, file: &mut FileContext) -> IdlcResult<()> {
        let mut builder = WireBuilder {
            renderer: self,
            pending: BTreeSet::new(),
            visited: BTreeSet::new(),
        };
        for endpoint in &mut file.endpoints {
            let (version, operation) = self
                .plan
                .versions
                .iter()
                .find_map(|version| {
                    version
                        .surface
                        .operations
                        .values()
                        .find(|operation| {
                            self.request_type(operation, &version.version.id) == endpoint.request
                        })
                        .map(|operation| (version, operation))
                })
                .ok_or_else(|| {
                    Layout::error(&endpoint.request, "missing served request surface")
                })?;
            let names = Names {
                version: &version.version.id,
                step_from: &version.version.id,
                upward: false,
                types: &version.surface.types,
                layout: &self.layout,
            };
            endpoint.parameters = operation
                .inputs
                .iter()
                .map(|member| {
                    let value = member.value.as_ref().ok_or_else(|| {
                        Layout::error(&member.name, "request parameter has no type")
                    })?;
                    Ok(WireField {
                        name: rust_ident(&member.name),
                        wire: member.name.clone(),
                        ty: names.value_type(value, false)?,
                        optional: value.optional,
                        schema: builder.schema(value, version, false)?,
                        flatten: None,
                    })
                })
                .collect::<IdlcResult<_>>()?;
        }
        while let Some((version, key)) = builder.pending.pop_first() {
            if !builder.visited.insert((version.clone(), key.clone())) {
                continue;
            }
            let version = self
                .plan
                .versions
                .iter()
                .find(|value| value.version.id == version)
                .ok_or_else(|| Layout::error(&key, "wire version has no public surface"))?;
            file.wire.push(builder.definition(version, &key)?);
        }
        Ok(())
    }
}

impl WireBuilder<'_> {
    fn names<'a>(&'a self, version: &'a VersionPlan) -> Names<'a> {
        Names {
            version: &version.version.id,
            step_from: &version.version.id,
            upward: false,
            types: &version.surface.types,
            layout: &self.renderer.layout,
        }
    }

    fn symbol(version: &str, key: &str) -> String {
        format!(
            "decode_wire_{}_{}",
            version.to_case(Case::Snake),
            key.replace("::", "_").to_case(Case::Snake)
        )
    }

    fn schema(
        &mut self,
        value: &TypeUse,
        version: &VersionPlan,
        recursive: bool,
    ) -> IdlcResult<WireValue> {
        let mut schema = self.ty(&value.ty, &value.scope, version)?;
        if let hir::Declarator::ArrayDeclarator(array) = &value.declarator {
            for bound in array.len.iter().rev() {
                schema = WireValue::Array {
                    inner: Box::new(schema),
                    length: self.names(version).bound(bound, &value.scope)?,
                };
            }
        }
        if recursive {
            schema = WireValue::Boxed(Box::new(schema));
        }
        if value.optional {
            schema = WireValue::Optional(Box::new(schema));
        }
        Ok(schema)
    }

    fn ty(
        &mut self,
        ty: &hir::TypeSpec,
        scope: &[String],
        version: &VersionPlan,
    ) -> IdlcResult<WireValue> {
        match ty {
            hir::TypeSpec::ScopedName(name) => {
                let key = self.names(version).resolve(name, scope)?;
                self.pending
                    .insert((version.version.id.clone(), key.clone()));
                Ok(WireValue::Named(Self::symbol(&version.version.id, &key)))
            }
            hir::TypeSpec::SequenceType(sequence) => Ok(WireValue::Sequence(Box::new(self.ty(
                &sequence.ty,
                scope,
                version,
            )?))),
            hir::TypeSpec::MapType(map) => {
                self.validate_map_key(&map.key, scope, version)?;
                Ok(WireValue::Map {
                    key: self.names(version).type_name(&map.key, scope)?,
                    value: Box::new(self.ty(&map.value, scope, version)?),
                })
            }
            hir::TypeSpec::TemplateType(value) => Err(Layout::error(
                &value.ident,
                "custom templates have no public wire validation contract",
            )),
            _ => Ok(WireValue::Scalar(self.names(version).type_name(ty, scope)?)),
        }
    }

    fn validate_map_key(
        &self,
        ty: &hir::TypeSpec,
        scope: &[String],
        version: &VersionPlan,
    ) -> IdlcResult<()> {
        let mut ty = ty;
        let mut scope = scope;
        let mut seen = BTreeSet::new();
        while let hir::TypeSpec::ScopedName(name) = ty {
            let key = self.names(version).resolve(name, scope)?;
            if !seen.insert(key.clone()) {
                return Err(Layout::error(&key, "recursive map key alias"));
            }
            self.renderer
                .layout
                .validate(&format!("{}::{key}", version.version.id))?;
            match version.surface.types.get(&key) {
                Some(TypeShape::Alias(value)) => {
                    ty = &value.ty;
                    scope = &value.scope;
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn definition(&mut self, version: &VersionPlan, key: &str) -> IdlcResult<WireDefinition> {
        self.renderer
            .layout
            .validate(&format!("{}::{key}", version.version.id))?;
        let shape = version
            .surface
            .types
            .get(key)
            .ok_or_else(|| Layout::error(key, "wire type is missing"))?;
        let mut definition = WireDefinition {
            name: Self::symbol(&version.version.id, key),
            ty: self.names(version).named(key),
            fields: Vec::new(),
            kind: "struct",
            inner: None,
            keys: Vec::new(),
            forbidden: Vec::new(),
            catch_all: false,
            construction: None,
        };
        match shape {
            TypeShape::Struct(members) => {
                self.renderer
                    .layout
                    .members(&format!("{}::{key}", version.version.id))?;
                let layouts = WireLayouts {
                    version: &version.version.id,
                    types: &version.surface.types,
                };
                let layout = layouts.layout(key, &mut BTreeSet::new())?;
                definition.keys = layout.keys.into_iter().collect();
                definition.catch_all = layout.catch_all;
                let mut reserved = BTreeSet::new();
                for other in &self.renderer.plan.versions {
                    if matches!(other.surface.types.get(key), Some(TypeShape::Struct(_))) {
                        reserved.extend(
                            WireLayouts {
                                version: &other.version.id,
                                types: &other.surface.types,
                            }
                            .layout(key, &mut BTreeSet::new())?
                            .keys,
                        );
                    }
                }
                definition.forbidden = reserved
                    .into_iter()
                    .filter(|key| !definition.keys.contains(key))
                    .collect();
                for member in members {
                    let value = member
                        .value
                        .as_ref()
                        .ok_or_else(|| Layout::error(key, "wire member has no value type"))?;
                    let flatten = if member.flatten {
                        let layout = layouts.value_layout(value, &mut BTreeSet::new())?;
                        Some(Flatten {
                            keys: layout.keys.into_iter().collect(),
                            receives_rest: layout.catch_all,
                        })
                    } else {
                        None
                    };
                    definition.fields.push(WireField {
                        name: rust_ident(&member.name),
                        wire: member.wire.clone(),
                        ty: self
                            .renderer
                            .field_type(member, &self.names(version), Some(key))?,
                        optional: value.optional,
                        schema: self.schema(
                            value,
                            version,
                            self.renderer
                                .member_recursive(member, &self.names(version), key)?,
                        )?,
                        flatten,
                    });
                }
                definition.construction = Some(
                    self.renderer.layout.construction(
                        &format!("{}::{key}", version.version.id),
                        definition
                            .fields
                            .iter()
                            .enumerate()
                            .map(|(index, field)| ConstructionField {
                                name: field.name.clone(),
                                value: ConstructionValue::Decoded { index },
                            })
                            .collect(),
                    )?,
                );
            }
            TypeShape::Enum(_) => {
                definition.kind = "enum";
            }
            TypeShape::Alias(value) => {
                definition.kind = "alias";
                definition.inner = Some(self.schema(value, version, false)?);
            }
            TypeShape::Union(members) => {
                let names = self.names(version);
                let union = self.renderer.union_definition(&names, key)?;
                let is_http = crate::generate::utils::has_annotation(&union.annotations, "http");
                definition.kind = if is_http { "http_union" } else { "union" };
                for member in members {
                    let value = member
                        .value
                        .as_ref()
                        .ok_or_else(|| Layout::error(key, "union wire case has no value type"))?;
                    let case = union
                        .case
                        .iter()
                        .find(|case| {
                            crate::generate::rust::util::declarator_name(&case.element.value)
                                == member.name
                        })
                        .ok_or_else(|| Layout::error(key, "union wire case has no public label"))?;
                    let name = self.renderer.union_case_name(case, is_http)?;
                    for label in &case.label {
                        let tag = match label {
                            hir::CaseLabel::Value(label) => {
                                union_serde_tag_string(&render_const(label))
                            }
                            hir::CaseLabel::Default => "default".into(),
                        };
                        definition.fields.push(WireField {
                            name: name.clone(),
                            wire: tag,
                            ty: self
                                .names(version)
                                .value_type(value, case.element.recursive)?,
                            optional: false,
                            schema: self.schema(value, version, case.element.recursive)?,
                            flatten: None,
                        });
                    }
                }
            }
            TypeShape::Unsupported(kind) => {
                return Err(Layout::error(
                    key,
                    &format!("{kind} has no public wire validation contract"),
                ));
            }
        }
        Ok(definition)
    }
}
