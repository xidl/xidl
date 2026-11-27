//! Canonical import contributions to a projected version's IDL fingerprint.

mod shared;

use self::shared::SharedImport;
use crate::error::{IdlcError, IdlcResult};
use crate::fmt::format_idl_source;
use crate::units::{Unit, UnitGraph};
use crate::version_diff::VersionDiff;
use std::collections::{BTreeMap, BTreeSet};
use xidl_parser::hir::printer::IdlPrinter;
use xidl_parser::hir::{Definition, ScopedName, Specification, VisitScopedNames};
use xidl_parser::parser::IncludeResolver;
use xidl_parser::versioning::VersioningRecord;

type ImportReference = (String, String);

pub(super) struct ImportedUnits {
    root_bindings: BTreeSet<String>,
    versioned: BTreeMap<String, VersionedImport>,
    shared: BTreeMap<String, SharedImport>,
}

struct VersionedImport {
    path: String,
    bindings: BTreeSet<String>,
    retained: Vec<Option<String>>,
    spec: Specification,
    record: VersioningRecord,
}

impl VersionedImport {
    fn project(&self, version: Option<&str>) -> IdlcResult<Specification> {
        match version {
            Some(version) => {
                let id = self.record.resolve_version(version)?;
                let projected = self.record.project(&self.spec, id)?;
                Ok(Specification(VersionDiff::unwrap_projected(projected)))
            }
            None => Ok(self.spec.clone()),
        }
    }
}

enum Reachable<'a> {
    Declaration(&'a SharedImport, usize),
    Version(&'a VersionedImport, Option<String>),
}

impl ImportedUnits {
    pub(super) fn load(graph: &UnitGraph, resolver: &mut dyn IncludeResolver) -> IdlcResult<Self> {
        let (root, dependencies) = graph
            .units()
            .split_last()
            .ok_or_else(|| IdlcError::fmt("version lock: reachability requires a root unit"))?;
        let mut result = Self {
            root_bindings: Self::bindings(root),
            versioned: BTreeMap::new(),
            shared: BTreeMap::new(),
        };
        let mut bound_versions: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
        let explicitly_referenced: BTreeSet<_> = graph
            .units()
            .iter()
            .flat_map(Unit::refs)
            .filter(|reference| reference.version().is_some())
            .map(|reference| reference.prefix())
            .collect();
        for import in graph.units().iter().flat_map(Unit::imports) {
            if let Some(version) = import.version() {
                bound_versions
                    .entry(import.prefix())
                    .or_default()
                    .insert(version.to_owned());
            }
        }
        for unit in dependencies {
            let spec = VersionDiff::lower_unit(graph, unit, resolver)?;
            let bindings = Self::bindings(unit);
            let path = unit.path().to_owned();
            if let Some(record) = VersioningRecord::analyze(&spec)? {
                let retained = match bound_versions.remove(unit.prefix()) {
                    Some(versions) => versions.into_iter().map(Some).collect(),
                    None if explicitly_referenced.contains(unit.prefix()) => Vec::new(),
                    None => vec![None],
                };
                result.versioned.insert(
                    unit.prefix().to_owned(),
                    VersionedImport {
                        path,
                        bindings,
                        retained,
                        spec,
                        record,
                    },
                );
            } else {
                result.shared.insert(
                    unit.prefix().to_owned(),
                    SharedImport::new(path, bindings, spec),
                );
            }
        }
        Ok(result)
    }

    fn bindings(unit: &Unit) -> BTreeSet<String> {
        unit.imports()
            .iter()
            .map(|import| import.prefix().to_owned())
            .collect()
    }

    fn external(bindings: &BTreeSet<String>, name: &ScopedName) -> Option<ImportReference> {
        let (prefix, path) = name.name.split_first()?;
        (!name.is_root && !path.is_empty() && bindings.contains(prefix))
            .then(|| (prefix.clone(), path.join("::")))
    }

    fn enqueue<'a>(&'a self, reference: &ImportReference, pending: &mut Vec<Reachable<'a>>) {
        let (prefix, path) = reference;
        if let Some(unit) = self.shared.get(prefix)
            && let Some(ids) = unit.index.by_name.get(path)
        {
            pending.extend(ids.iter().map(|&id| Reachable::Declaration(unit, id)));
        }
        if let Some(unit) = self.versioned.get(prefix)
            && let Some((version, _)) = path.split_once("::")
        {
            pending.push(Reachable::Version(unit, Some(version.to_owned())));
        }
    }

    fn enqueue_external<'a>(
        &'a self,
        bindings: &BTreeSet<String>,
        definitions: &mut [Definition],
        pending: &mut Vec<Reachable<'a>>,
    ) {
        for definition in definitions {
            definition.visit_scoped_names_with_annotations(&mut |name| {
                if let Some(reference) = Self::external(bindings, name) {
                    self.enqueue(&reference, pending);
                }
            });
        }
    }

    pub(super) fn canonical(
        &self,
        projection: &mut [Definition],
    ) -> IdlcResult<BTreeMap<String, String>> {
        let mut pending = Vec::new();
        for unit in self.versioned.values() {
            pending.extend(
                unit.retained
                    .iter()
                    .cloned()
                    .map(|version| Reachable::Version(unit, version)),
            );
        }
        self.enqueue_external(&self.root_bindings, projection, &mut pending);

        let mut reached: BTreeMap<&str, BTreeSet<usize>> = BTreeMap::new();
        let mut versioned: BTreeMap<&str, BTreeMap<Option<String>, String>> = BTreeMap::new();
        while let Some(next) = pending.pop() {
            match next {
                Reachable::Declaration(unit, id) => {
                    if !reached.entry(unit.path.as_str()).or_default().insert(id) {
                        continue;
                    }
                    let declaration = &unit.index.declarations[id];
                    if let Some(container) = declaration.container {
                        pending.push(Reachable::Declaration(unit, container));
                    }
                    for name in &declaration.references {
                        if let Some(reference) = Self::external(&unit.bindings, name) {
                            self.enqueue(&reference, &mut pending);
                        } else if let Some(ids) = unit.index.resolve(&declaration.scope, name) {
                            pending.extend(ids.iter().map(|&id| Reachable::Declaration(unit, id)));
                        }
                    }
                }
                Reachable::Version(unit, version) => {
                    let sections = versioned.entry(unit.path.as_str()).or_default();
                    if let std::collections::btree_map::Entry::Vacant(entry) =
                        sections.entry(version)
                    {
                        let selector = entry.key().as_deref();
                        let mut spec = unit.project(selector)?;
                        self.enqueue_external(&unit.bindings, &mut spec.0, &mut pending);
                        let section = format_idl_source(&IdlPrinter::render(&spec))?;
                        let section = match selector {
                            Some(selector) => format!("@version({selector})\n{section}"),
                            None => section,
                        };
                        entry.insert(section);
                    }
                }
            }
        }

        let mut rendered: BTreeMap<_, _> = versioned
            .into_iter()
            .map(|(path, sections)| (path.to_owned(), sections.into_values().collect()))
            .collect();
        for unit in self.shared.values() {
            if let Some(declarations) = reached.get(unit.path.as_str()) {
                rendered.insert(unit.path.clone(), unit.render(declarations)?);
            }
        }
        Ok(rendered)
    }
}
