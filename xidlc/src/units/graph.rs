use super::error::ImportError;
use super::interface::UnitInterface;
use super::path::ImportPath;
use super::refs::PreparedSource;
use super::scan::{DeclaredImports, ImportDecl};
use super::spec::{ImportSpec, Unit};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use xidl_parser::semantic::IntegerImports;

/// Entry file a directory import resolves to.
const INDEX_FILE: &str = "index.idl";

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
            units: builder.units,
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
                    import.insert(
                        "members".to_string(),
                        interface.members().iter().cloned().collect(),
                    );
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
                let interface = UnitInterface::of(target.source(), target.file(), &self.mounts)?;
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

/// Depth first walk that turns import declarations into units.
struct UnitBuilder<'a> {
    mounts: &'a BTreeMap<String, PathBuf>,
    units: Vec<Unit>,
    by_file: BTreeMap<PathBuf, usize>,
    prefixes: BTreeMap<String, PathBuf>,
    stack: Vec<PathBuf>,
}

impl<'a> UnitBuilder<'a> {
    /// Creates a builder that resolves imports against `mounts`.
    fn new(mounts: &'a BTreeMap<String, PathBuf>) -> Self {
        Self {
            mounts,
            units: Vec::new(),
            by_file: BTreeMap::new(),
            prefixes: BTreeMap::new(),
            stack: Vec::new(),
        }
    }

    /// Adds one compiler input under the prefix its file name provides.
    fn add_root(&mut self, path: &Path) -> Result<(), ImportError> {
        let prefix = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| ImportError::InvalidPath {
                importer: path.to_path_buf(),
                import: path.display().to_string(),
                reason: "the file name provides no reference prefix".to_string(),
            })?;
        let unit_path = Self::root_unit_path(path);
        self.visit(path.to_path_buf(), prefix.to_string(), unit_path)
    }

    /// Path a root input is known by: the path it was given, relative to the
    /// working directory, and its file name when it was given as an absolute
    /// path, so a target never lays output out beside the source.
    fn root_unit_path(path: &Path) -> String {
        if path.is_absolute() {
            return path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
        }
        Self::strip_leading_dot(path.display().to_string())
    }

    /// Registers `file` as the unit named `prefix`, resolving its imports.
    fn visit(&mut self, file: PathBuf, prefix: String, path: String) -> Result<(), ImportError> {
        let identity = Self::identity(&file);
        if self.stack.contains(&identity) {
            let mut chain = self.stack.clone();
            chain.push(identity);
            return Err(ImportError::Cycle { chain });
        }
        if let Some(index) = self.by_file.get(&identity).copied() {
            let existing = self.units[index].prefix();
            if existing != prefix {
                return Err(ImportError::UnitAliased {
                    file,
                    first: existing.to_string(),
                    second: prefix,
                });
            }
            return Ok(());
        }

        let source = Self::read(&file)?;
        let imports = self.resolve_imports(&file, &source)?;
        let prepared = PreparedSource::of(&source, &Self::bindings(&imports))?;

        self.stack.push(identity.clone());
        for import in &imports {
            self.visit(
                import.file().to_path_buf(),
                import.prefix().to_string(),
                import.unit_path().to_string(),
            )?;
        }
        self.stack.pop();

        self.register_prefix(&prefix, &file)?;
        self.by_file.insert(identity, self.units.len());
        self.units
            .push(Unit::new(file, prefix, imports, prepared, path));
        Ok(())
    }

    /// Drops the `./` a written path may start with.
    fn strip_leading_dot(path: String) -> String {
        path.strip_prefix("./").map(str::to_string).unwrap_or(path)
    }

    /// Version each import bound, keyed by the reference prefix it uses.
    fn bindings(imports: &[ImportSpec]) -> BTreeMap<String, Option<String>> {
        imports
            .iter()
            .map(|import| {
                (
                    import.prefix().to_string(),
                    import.version().map(str::to_string),
                )
            })
            .collect()
    }

    /// Resolves every import declaration of one file.
    fn resolve_imports(&self, file: &Path, source: &str) -> Result<Vec<ImportSpec>, ImportError> {
        let declared = DeclaredImports::of(source, file, self.mounts)?;
        let mut imports: Vec<ImportSpec> = Vec::new();
        for declaration in declared.into_decls() {
            let raw = match declaration {
                ImportDecl::Path(path) => path,
                ImportDecl::Scoped(scope) => {
                    return Err(ImportError::Unquoted {
                        importer: file.to_path_buf(),
                        scope,
                    });
                }
            };
            let import = self.resolve_import(file, &raw)?;
            let duplicate = imports.iter().any(|existing| {
                existing.prefix() == import.prefix() && existing.file() == import.file()
            });
            if !duplicate {
                imports.push(import);
            }
        }
        Ok(imports)
    }

    /// Resolves one import path relative to its importer or a mount.
    fn resolve_import(&self, importer: &Path, raw: &str) -> Result<ImportSpec, ImportError> {
        let path = ImportPath::parse(raw);
        let prefix = path.prefix().ok_or_else(|| ImportError::InvalidPath {
            importer: importer.to_path_buf(),
            import: raw.to_string(),
            reason: "the path has no name to reference the unit by".to_string(),
        })?;
        let (file, unit_path) = self.resolve_target(importer, path.target())?;
        Ok(ImportSpec::new(
            prefix,
            path.version().map(str::to_string),
            raw.to_string(),
            file,
            unit_path,
        ))
    }

    /// Turns an import target into the IDL file it names.
    fn resolve_target(
        &self,
        importer: &Path,
        target: &str,
    ) -> Result<(PathBuf, String), ImportError> {
        let (root, rest, unit_path) = self.split_mount(importer, target);
        let candidate = root.join(rest);
        if candidate.is_dir() {
            let entry = candidate.join(INDEX_FILE);
            if entry.is_file() {
                return Ok((entry, format!("{unit_path}/{INDEX_FILE}")));
            }
            return Err(ImportError::Unresolved {
                importer: importer.to_path_buf(),
                import: target.to_string(),
                reason: format!(
                    "{} is a directory without an {INDEX_FILE}",
                    candidate.display()
                ),
            });
        }
        if candidate.is_file() {
            return Ok((candidate, unit_path));
        }
        Err(ImportError::Unresolved {
            importer: importer.to_path_buf(),
            import: target.to_string(),
            reason: format!("no IDL file at {}", candidate.display()),
        })
    }

    /// Picks the directory an import path is resolved against.
    ///
    /// A first segment that names a mount resolves inside that package, which
    /// mirrors how `#include "<name>/<file>"` reaches a dependency.
    fn split_mount(&self, importer: &Path, target: &str) -> (PathBuf, String, String) {
        let (name, rest) = match target.split_once('/') {
            Some((name, rest)) => (name, rest),
            None => (target, ""),
        };
        match self.mounts.get(name) {
            Some(root) => {
                let rest = rest.to_string();
                let unit_path = Self::join_unit_path(name, &rest);
                (root.clone(), rest, unit_path)
            }
            None => (
                importer
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .to_path_buf(),
                target.to_string(),
                Self::strip_leading_dot(target.to_string()),
            ),
        }
    }

    /// VFS path of a dependency that a mount resolved.
    fn join_unit_path(mount: &str, rest: &str) -> String {
        let rest = rest.trim_matches('/');
        if rest.is_empty() {
            mount.to_string()
        } else {
            format!("{mount}/{rest}")
        }
    }

    /// Rejects a prefix already claimed by another unit.
    fn register_prefix(&mut self, prefix: &str, file: &Path) -> Result<(), ImportError> {
        if let Some(first) = self.prefixes.get(prefix) {
            return Err(ImportError::PrefixConflict {
                prefix: prefix.to_string(),
                first: first.clone(),
                second: file.to_path_buf(),
            });
        }
        self.prefixes.insert(prefix.to_string(), file.to_path_buf());
        Ok(())
    }

    /// Reads one unit, naming it when it cannot be loaded.
    fn read(file: &Path) -> Result<String, ImportError> {
        fs::read_to_string(file).map_err(|source| ImportError::Io {
            path: file.to_path_buf(),
            source,
        })
    }

    /// Identity of a unit: the canonical path when the filesystem knows it.
    fn identity(file: &Path) -> PathBuf {
        fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf())
    }
}
