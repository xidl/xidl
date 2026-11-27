use super::wire::WireContract;
use crate::driver::{File, Generator};
use crate::error::{IdlcError, IdlcResult};
use crate::library::{MOUNT_PROPERTY, ResolvedInputs};
use crate::openapi::OpenApiFilename;
use crate::units::UnitGraph;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use xidl_parser::versioning::API_VERSION_PROPERTY;

/// Generates OpenAPI artifacts in memory with the ordinary generation pipeline.
pub(super) struct OpenApiFingerprints<'a> {
    generator: Generator,
    graph: &'a UnitGraph,
    props: HashMap<String, Value>,
    imports: BTreeMap<String, Value>,
}

impl<'a> OpenApiFingerprints<'a> {
    /// Prepares the generator with the same inputs used for IDL projection.
    pub(super) fn new(graph: &'a UnitGraph, resolved: &ResolvedInputs) -> IdlcResult<Self> {
        if !cfg!(feature = "gen-openapi") {
            return Err(IdlcError::fmt(
                "version lock: the OpenAPI artifact requires the gen-openapi feature",
            ));
        }
        let mut props = HashMap::new();
        if !resolved.mounts().is_empty() {
            props.insert(MOUNT_PROPERTY.to_owned(), resolved.mount_property());
        }
        Ok(Self {
            generator: Generator::new("openapi".to_owned()),
            graph,
            props,
            imports: BTreeMap::new(),
        })
    }

    /// Hashes the generated document for one version and the schemas it references.
    pub(super) fn fingerprint(&mut self, version: &str) -> IdlcResult<String> {
        let root = self.graph.units().last().ok_or_else(|| {
            IdlcError::fmt("version lock: OpenAPI generation requires one root unit")
        })?;
        let mut props = self.props.clone();
        props.insert(API_VERSION_PROPERTY.to_owned(), version.into());
        let files = self.generator.generate_unit(self.graph, root, props)?;
        let [file] = files.as_slice() else {
            return Err(IdlcError::fmt(format!(
                "version lock: expected one OpenAPI artifact for version '{version}'"
            )));
        };
        WireContract::fingerprint(&Self::decode(file)?, |document, pointer| {
            self.schema(document, pointer)
        })
    }

    fn schema(&mut self, document: &str, pointer: &str) -> IdlcResult<Option<Value>> {
        if !self.imports.contains_key(document) {
            // Resolved references can select a different member from the import's default.
            let mut matches = self.graph.units().iter().filter_map(|unit| {
                OpenApiFilename::version_for(document, unit.prefix()).map(|version| (unit, version))
            });
            let Some((unit, version)) = matches.next() else {
                return Ok(None);
            };
            if matches.next().is_some() {
                return Err(IdlcError::fmt(format!(
                    "version lock: OpenAPI artifact '{document}' matches multiple units"
                )));
            }
            let mut props = self.props.clone();
            if let Some(version) = version {
                props.insert(API_VERSION_PROPERTY.to_owned(), version.into());
            }
            for file in self.generator.generate_unit(self.graph, unit, props)? {
                self.imports
                    .insert(file.path().to_owned(), Self::decode(&file)?);
            }
        }
        Ok(self
            .imports
            .get(document)
            .and_then(|document| document.pointer(pointer))
            .cloned())
    }

    fn decode(file: &File) -> IdlcResult<Value> {
        serde_json::from_str(file.content()).map_err(|error| {
            IdlcError::fmt(format!(
                "version lock: invalid generated OpenAPI in '{}': {error}",
                file.path()
            ))
        })
    }
}
