use super::refs::{PreparedSource, UnitRef};
use std::path::{Path, PathBuf};

/// One resolved import of a unit.
#[derive(Debug, Clone)]
pub struct ImportSpec {
    prefix: String,
    version: Option<String>,
    path: String,
    file: PathBuf,
    unit_path: String,
}

impl ImportSpec {
    /// Creates a resolved import.
    pub(super) fn new(
        prefix: String,
        version: Option<String>,
        path: String,
        file: PathBuf,
        unit_path: String,
    ) -> Self {
        Self {
            prefix,
            version,
            path,
            file,
            unit_path,
        }
    }

    /// Reference prefix the importing unit writes for this unit.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// API version bound by the import, when it carried a selector.
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// Import path as written in the source.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// File the import resolved to.
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Path the imported unit has inside the mount or directory tree.
    ///
    /// A dependency resolved through the `timestamp` mount of
    /// `#include "timestamp/lib.idl"` reports `timestamp/lib.idl`, and a
    /// relative import reports the path as written.
    pub fn unit_path(&self) -> &str {
        &self.unit_path
    }

    /// Serializes the import for the generator property.
    fn to_json(&self) -> serde_json::Value {
        let mut value = serde_json::Map::new();
        value.insert("prefix".into(), self.prefix.clone().into());
        value.insert("path".into(), self.path.clone().into());
        value.insert("unit".into(), self.unit_path.clone().into());
        value.insert(
            "file".into(),
            self.file.to_string_lossy().into_owned().into(),
        );
        if let Some(version) = &self.version {
            value.insert("version".into(), version.clone().into());
        }
        serde_json::Value::Object(value)
    }
}

/// One compilation unit of an import graph.
#[derive(Debug, Clone)]
pub struct Unit {
    file: PathBuf,
    prefix: String,
    imports: Vec<ImportSpec>,
    prepared: PreparedSource,
    path: String,
}

impl Unit {
    /// Creates a unit from its file, prefix, imports, source and references.
    pub(super) fn new(
        file: PathBuf,
        prefix: String,
        imports: Vec<ImportSpec>,
        prepared: PreparedSource,
        path: String,
    ) -> Self {
        Self {
            file,
            prefix,
            imports,
            prepared,
            path,
        }
    }

    /// File the unit is read from.
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Reference prefix of the unit, such as `timestamp`.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Path the unit has inside the mount or directory tree.
    ///
    /// A unit imported as `timestamp` reports `timestamp/index.idl`, and a root
    /// input reports the path it was given, so a target that mirrors the tree
    /// can lay its output out the same way.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Imports this unit declares, resolved and deduplicated.
    pub fn imports(&self) -> &[ImportSpec] {
        &self.imports
    }

    /// Import that binds `prefix`, when this unit imports that name.
    pub fn import(&self, prefix: &str) -> Option<&ImportSpec> {
        self.imports.iter().find(|import| import.prefix() == prefix)
    }

    /// Source handed to generation, with bound versions spelled out.
    pub fn source(&self) -> &str {
        self.prepared.source()
    }

    /// References this unit makes into other units.
    pub fn refs(&self) -> &[UnitRef] {
        self.prepared.refs()
    }

    /// Input path handed to generation.
    ///
    /// A unit named after a directory (`import "pkg"`) is generated as
    /// `pkg.<ext>` so the artifact matches the prefix every reference uses.
    pub fn generation_input(&self) -> PathBuf {
        let stem = self.file.file_stem().and_then(|stem| stem.to_str());
        if stem == Some(self.prefix.as_str()) {
            return self.file.clone();
        }
        self.file.with_file_name(format!("{}.idl", self.prefix))
    }

    /// The `xidl_imports` property a generator target reads.
    pub fn import_property(&self) -> serde_json::Value {
        serde_json::Value::Array(self.imports.iter().map(ImportSpec::to_json).collect())
    }
}
