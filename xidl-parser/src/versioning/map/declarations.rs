//! Source declaration identities, collected before lifecycle projection.

use super::model::{OperationSurface, TypeSurface};
use crate::error::{ParseError, ParserResult};
use crate::hir::{
    Annotation, ParamDcl, Specification, effective_wire_name, field_rename, is_skipped,
};
use crate::rest_hir::HttpOperationSource;
use crate::rest_hir::project_params::sources::ParameterSources;
use crate::versioning::lifecycle::{Availability, Site, SiteKind, VersionVisitor, availability};
use crate::versioning::timeline::{VersionId, VersionTimeline};
use serde_json::Value;
use std::collections::BTreeMap;

struct Declaration {
    id: usize,
    kind: SiteKind,
    parameter: Option<ParamDcl>,
    availability: Availability,
    wire_name: String,
}

struct DeclarationQuery<'a> {
    path: &'a str,
    kind: SiteKind,
    wire_name: Option<&'a str>,
    parameter_binding: Option<(&'a ParameterSources, &'a str)>,
}

impl Declaration {
    fn matches(&self, query: &DeclarationQuery<'_>) -> ParserResult<bool> {
        if self.kind != query.kind || query.wire_name.is_some_and(|wire| wire != self.wire_name) {
            return Ok(false);
        }
        if let Some((sources, location)) = query.parameter_binding {
            let Some(parameter) = &self.parameter else {
                return Ok(false);
            };
            let projected = sources.request(parameter).map_err(|reason| {
                ParseError::Message(format!("version map: {}: {reason}", query.path))
            })?;
            return Ok(projected.is_some_and(|param| {
                super::collect::render::render_param_kind(param.kind) == location
            }));
        }
        Ok(true)
    }

    fn token(&self) -> String {
        format!("d{}", self.id)
    }
}

/// Source identities selected by declaration kind, availability and wire binding.
pub(super) struct SourceDeclarations<'a> {
    timeline: &'a VersionTimeline,
    next_id: usize,
    declarations: BTreeMap<String, Vec<Declaration>>,
    owners: BTreeMap<String, Vec<Annotation>>,
}

impl<'a> SourceDeclarations<'a> {
    /// Indexes declarations before lifecycle projection removes their origin.
    pub(super) fn collect(
        spec: &Specification,
        timeline: &'a VersionTimeline,
    ) -> ParserResult<Self> {
        let mut index = Self {
            timeline,
            next_id: 0,
            declarations: BTreeMap::new(),
            owners: BTreeMap::new(),
        };
        index.walk(&spec.0, "", Availability::full())?;
        Ok(index)
    }

    /// Attaches direct member identities to projected type descriptors.
    pub(super) fn stamp_types(
        &self,
        types: &mut BTreeMap<String, TypeSurface>,
        version: VersionId,
    ) -> ParserResult<()> {
        for (type_name, surface) in types {
            for (wire_name, member) in &mut surface.members {
                let name = member.get("name").and_then(Value::as_str).ok_or_else(|| {
                    ParseError::Message(format!(
                        "version map: {type_name} has a member descriptor without a name"
                    ))
                })?;
                let path = format!("{type_name}::{name}");
                let kind = match surface.kind.as_str() {
                    "enum" => SiteKind::Enumerator,
                    "bitmask" => SiteKind::BitmaskValue,
                    "bitset" => SiteKind::Bitfield,
                    _ => SiteKind::Member,
                };
                let id = self
                    .identity(
                        DeclarationQuery {
                            path: &path,
                            kind,
                            wire_name: Some(wire_name),
                            parameter_binding: None,
                        },
                        version,
                    )?
                    .token();
                member.insert("declaration_id".into(), Value::String(id));
            }
        }
        Ok(())
    }

    /// Attaches operation and request/response parameter identities.
    pub(super) fn stamp_operations(
        &self,
        operations: &mut BTreeMap<String, OperationSurface>,
        version: VersionId,
    ) -> ParserResult<()> {
        for (key, surface) in operations {
            let (interface, operation) = key.rsplit_once('.').ok_or_else(|| {
                ParseError::Message(format!("version map: invalid operation key {key}"))
            })?;
            let origin = match surface.source {
                HttpOperationSource::Method => Some(operation),
                HttpOperationSource::AttributeGet => operation.strip_prefix("get_attribute_"),
                HttpOperationSource::AttributeSet => operation.strip_prefix("set_attribute_"),
                HttpOperationSource::AttributeWatch => operation.strip_prefix("watch_attribute_"),
            }
            .ok_or_else(|| {
                ParseError::Message(format!(
                    "version map: generated operation {key} has no source attribute name"
                ))
            })?;
            let path = format!("{interface}::{origin}");
            let kind = if surface.source == HttpOperationSource::Method {
                SiteKind::Operation
            } else {
                SiteKind::Attribute
            };
            surface.descriptor.insert(
                "declaration_id".into(),
                Value::String(
                    self.identity(
                        DeclarationQuery {
                            path: &path,
                            kind,
                            wire_name: None,
                            parameter_binding: None,
                        },
                        version,
                    )?
                    .token(),
                ),
            );
            for parameter in surface.params.values_mut() {
                let name = parameter
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        ParseError::Message(format!(
                            "version map: operation {key} has a parameter without a name"
                        ))
                    })?;
                let parameter_path = if surface.source == HttpOperationSource::Method {
                    format!("{path}::{name}")
                } else {
                    path.clone()
                };
                let wire_name = parameter
                    .get("wire_name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        ParseError::Message(format!(
                            "version map: operation {key} has a parameter without a wire name"
                        ))
                    })?;
                let location = parameter
                    .get("location")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        ParseError::Message(format!(
                            "version map: operation {key} has a parameter without a location"
                        ))
                    })?;
                let query = DeclarationQuery {
                    path: &parameter_path,
                    kind: if surface.source == HttpOperationSource::Method {
                        SiteKind::Parameter
                    } else {
                        SiteKind::Attribute
                    },
                    wire_name: (surface.source == HttpOperationSource::Method).then_some(wire_name),
                    parameter_binding: (surface.source == HttpOperationSource::Method)
                        .then_some((&surface.parameter_sources, location)),
                };
                let declaration = self.identity(query, version)?;
                if !parameter.contains_key("optional") {
                    let optional = if let Some(source) = &declaration.parameter {
                        surface
                            .parameter_sources
                            .request(source)
                            .map_err(|reason| {
                                ParseError::Message(format!(
                                    "version map: operation {key}: {reason}"
                                ))
                            })?
                            .is_some_and(|param| param.optional)
                    } else {
                        false
                    };
                    parameter.insert("optional".into(), Value::Bool(optional));
                }
                parameter.insert("declaration_id".into(), Value::String(declaration.token()));
            }
            if surface.source == HttpOperationSource::Method {
                if let Some(parameters) =
                    self.response_parameters(&path, &surface.parameter_sources, version)?
                {
                    surface
                        .descriptor
                        .insert("response_parameters".into(), parameters);
                }
            }
        }
        Ok(())
    }

    fn response_parameters(
        &self,
        operation: &str,
        sources: &ParameterSources,
        version: VersionId,
    ) -> ParserResult<Option<Value>> {
        let prefix = format!("{operation}::");
        let mut parameters: BTreeMap<String, Vec<(usize, Value)>> = BTreeMap::new();
        for (path, declarations) in self.declarations.range(prefix.clone()..) {
            if !path.starts_with(&prefix) {
                break;
            }
            for declaration in declarations {
                if !declaration.availability.contains(version)
                    || declaration.kind != SiteKind::Parameter
                {
                    continue;
                }
                let Some(source) = &declaration.parameter else {
                    continue;
                };
                let Some(projected) = sources.response(source).map_err(|reason| {
                    ParseError::Message(format!("version map: {path}: {reason}"))
                })?
                else {
                    continue;
                };
                let location = super::collect::render::render_param_kind(projected.kind);
                let key = format!("{location}:{}", projected.wire_name);
                let descriptor = serde_json::json!({
                    "declaration_id": declaration.token(),
                    "name": projected.name,
                    "location": location,
                    "wire_name": projected.wire_name,
                });
                parameters
                    .entry(key)
                    .or_default()
                    .push((declaration.id, descriptor));
            }
        }
        if parameters.is_empty() {
            return Ok(None);
        }
        let parameters = parameters
            .into_iter()
            .map(|(key, mut declarations)| {
                declarations.sort_by_key(|(id, _)| *id);
                (
                    key,
                    Value::Array(
                        declarations
                            .into_iter()
                            .map(|(_, descriptor)| descriptor)
                            .collect(),
                    ),
                )
            })
            .collect();
        Ok(Some(Value::Object(parameters)))
    }

    fn identity(
        &self,
        query: DeclarationQuery<'_>,
        version: VersionId,
    ) -> ParserResult<&Declaration> {
        let mut matched = None;
        for declaration in self.declarations.get(query.path).into_iter().flatten() {
            if !declaration.availability.contains(version) || !declaration.matches(&query)? {
                continue;
            }
            if matched.is_some() {
                return Err(ParseError::Message(format!(
                    "version map: source identity for {} is ambiguous in version {}",
                    query.path,
                    version.index()
                )));
            }
            matched = Some(declaration);
        }
        matched.ok_or_else(|| {
            ParseError::Message(format!(
                "version map: source identity for {} is missing in version {}",
                query.path,
                version.index()
            ))
        })
    }
}

impl VersionVisitor for SourceDeclarations<'_> {
    fn enter(&mut self, site: &Site<'_>, parent: Availability) -> ParserResult<Availability> {
        let own = availability(site, parent, self.timeline)?;
        if site.kind == SiteKind::Type {
            self.owners
                .insert(site.path.clone(), site.annotations.to_vec());
        }
        if matches!(
            site.kind,
            SiteKind::Member
                | SiteKind::Enumerator
                | SiteKind::BitmaskValue
                | SiteKind::Bitfield
                | SiteKind::Parameter
                | SiteKind::Operation
                | SiteKind::Attribute
        ) {
            let names = site
                .path
                .strip_prefix(&format!("{}::", site.parent_path))
                .unwrap_or(&site.path);
            for name in names.split(", ") {
                let id = self.next_id;
                self.next_id += 1;
                if matches!(
                    site.kind,
                    SiteKind::Member
                        | SiteKind::Enumerator
                        | SiteKind::BitmaskValue
                        | SiteKind::Parameter
                ) && is_skipped(site.annotations)
                {
                    continue;
                }
                let path = if site.parent_path.is_empty() {
                    name.to_string()
                } else {
                    format!("{}::{name}", site.parent_path)
                };
                let annotations = self
                    .owners
                    .get(&site.parent_path)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let wire_name = match site.kind {
                    SiteKind::Parameter => {
                        field_rename(site.annotations).unwrap_or_else(|| name.into())
                    }
                    SiteKind::Bitfield => name.into(),
                    _ => effective_wire_name(name, site.annotations, annotations),
                };
                self.declarations
                    .entry(path)
                    .or_default()
                    .push(Declaration {
                        id,
                        kind: site.kind,
                        parameter: site.parameter.cloned(),
                        availability: own,
                        wire_name,
                    });
            }
        }
        Ok(own)
    }
}
