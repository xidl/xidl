use super::builder::UnitBuilder;
use super::error::ImportError;
use super::interface::UnitInterface;
use super::spec::Unit;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use xidl_parser::semantic::IntegerImports;

/// The import graph of one compiler run, in dependency order.
#[derive(Debug)]
pub struct UnitGraph {
    units: Vec<Unit>,
    mounts: BTreeMap<String, PathBuf>,
    interfaces: BTreeMap<String, UnitInterface>,
}

impl UnitGraph {
    /// Resolves `inputs` into units by following their `import` declarations.
    ///
    /// Every input is a unit. Each import adds the unit it names, so the graph
    /// holds every transitively reachable file exactly once. Imports resolve
    /// against `mounts` first and against the importing file's directory
    /// afterwards, and a directory import resolves to its `index.idl`.
    ///
    /// # Errors
    ///
    /// Returns [`ImportError`] when an input cannot be read or parsed, when an
    /// import does not resolve, when two units claim one reference prefix, or
    /// when the import graph contains a cycle.
    pub fn build(
        inputs: &[PathBuf],
        mounts: &BTreeMap<String, PathBuf>,
    ) -> Result<Self, ImportError> {
        let mut builder = UnitBuilder::new(mounts);
        for input in inputs {
            builder.add_root(input)?;
        }
        let mut graph = Self {
            units: builder.into_units(),
            mounts: mounts.clone(),
            interfaces: BTreeMap::new(),
        };
        graph.collect_interfaces()?;
        graph.validate()?;
        Ok(graph)
    }

    /// Units in dependency order: every unit follows the units it imports.
    pub fn units(&self) -> &[Unit] {
        &self.units
    }

    /// Generator import metadata, including declared member identities.
    pub fn import_property(&self, unit: &Unit) -> serde_json::Value {
        let mut property = unit.import_property();
        if let Some(imports) = property.as_array_mut() {
            for import in imports {
                let interface = import
                    .get("prefix")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|prefix| self.interfaces.get(prefix));
                if let Some(interface) = interface
                    && let Some(import) = import.as_object_mut()
                {
                    interface.enrich_import(import);
                }
            }
        }
        property
    }

    /// Supplies only the imported declaration contexts reachable from `unit`.
    pub fn integer_imports(&self, unit: &Unit) -> IntegerImports {
        let bindings = unit
            .imports()
            .iter()
            .map(|import| import.prefix().to_owned());
        let mut reachable: BTreeSet<String> = bindings.clone().collect();
        let mut imports = IntegerImports::new(bindings);
        for target in self.units.iter().rev() {
            if !reachable.contains(target.prefix()) {
                continue;
            }
            reachable.extend(
                target
                    .imports()
                    .iter()
                    .map(|import| import.prefix().to_owned()),
            );
            if let Some(interface) = self.interfaces.get(target.prefix()) {
                imports.bind(
                    target.prefix(),
                    interface.integer_exports(),
                    target
                        .imports()
                        .iter()
                        .map(|import| import.prefix().to_owned()),
                );
            }
        }
        imports
    }

    fn collect_interfaces(&mut self) -> Result<(), ImportError> {
        let imported: BTreeSet<_> = self
            .units
            .iter()
            .flat_map(|unit| unit.imports().iter().map(|import| import.prefix()))
            .collect();
        for target in &self.units {
            if imported.contains(target.prefix()) {
                let interface =
                    UnitInterface::of(target.typed_ast().clone(), target.file(), &self.mounts)?;
                self.interfaces
                    .insert(target.prefix().to_owned(), interface);
            }
        }
        Ok(())
    }

    /// Checks every reference a unit makes against the unit it names.
    ///
    /// A reference into an imported unit must name a version that unit
    /// declares and a declaration that exists at that version, so a stale or
    /// misspelled cross-unit type fails here instead of inside a target.
    fn validate(&self) -> Result<(), ImportError> {
        for unit in &self.units {
            for reference in unit.refs() {
                if unit.import(reference.prefix()).is_none() {
                    continue;
                }
                let Some(target) = self
                    .units
                    .iter()
                    .find(|candidate| candidate.prefix() == reference.prefix())
                else {
                    continue;
                };
                let Some(interface) = self.interfaces.get(reference.prefix()) else {
                    continue;
                };
                interface.check(unit, target, reference)?;
            }
        }
        Ok(())
    }
}
