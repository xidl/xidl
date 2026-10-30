//! Go packages the units a file imports live in.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use xidl_parser::hir;
use xidl_parser::hir::ParserProperties;

use super::definition::go_export_name;
use crate::generate::utils::go_package_name;

/// One Go package a file reaches.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(crate) struct GoImport {
    /// Package name references use.
    pub(crate) name: String,
    /// Import path of the package.
    pub(crate) path: String,
}

/// The package each imported unit is reached through.
///
/// A unit generated beside the importing file shares its package, so its types
/// stay unqualified. Any other unit gets a package named after the directory it
/// was generated into, which is the directory its IDL path names.
#[derive(Debug, Default)]
pub(crate) struct GoPackages {
    packages: BTreeMap<String, Option<GoImport>>,
    referenced: RefCell<BTreeSet<String>>,
    runtime: RefCell<BTreeMap<String, GoImport>>,
}

impl GoPackages {
    /// Reads the packages of the units `properties` imports.
    pub(crate) fn of(properties: &ParserProperties) -> Self {
        let current = properties
            .get(crate::units::UNIT_PATH_PROPERTY)
            .and_then(serde_json::Value::as_str);
        let Some(imports) = properties
            .get(crate::units::IMPORT_PROPERTY)
            .and_then(serde_json::Value::as_array)
        else {
            return Self::default();
        };
        let mut packages = BTreeMap::new();
        for import in imports {
            let prefix = import.get("prefix").and_then(serde_json::Value::as_str);
            let unit = import.get("unit").and_then(serde_json::Value::as_str);
            let (Some(prefix), Some(unit)) = (prefix, unit) else {
                continue;
            };
            let package = match Self::package_path(unit) {
                Some(path) if Some(path.as_str()) != Self::package_path_of(current) => {
                    Some(GoImport {
                        name: go_package_name(path.rsplit('/').next().unwrap_or(&path)),
                        path,
                    })
                }
                _ => None,
            };
            packages.insert(prefix.to_string(), package);
        }
        Self {
            packages,
            referenced: RefCell::default(),
            runtime: RefCell::default(),
        }
    }

    /// Reference to `value`, when it names an imported unit.
    pub(crate) fn reference(&self, value: &hir::ScopedName) -> Option<String> {
        let (first, rest) = value.name.split_first()?;
        if rest.is_empty() || value.is_root {
            return None;
        }
        let (ident, modules) = rest.split_last()?;
        let name = go_export_name(modules, ident);
        match self.packages.get(first)? {
            Some(package) => {
                self.referenced.borrow_mut().insert(package.path.clone());
                Some(format!("{}.{name}", package.name))
            }
            None => Some(name),
        }
    }

    /// Packages referenced during rendering, sorted by import path.
    pub(crate) fn imports(&self) -> Vec<GoImport> {
        let referenced = self.referenced.borrow();
        let mut imports: Vec<GoImport> = self
            .packages
            .values()
            .flatten()
            .filter(|package| referenced.contains(&package.path))
            .cloned()
            .collect();
        imports.extend(self.runtime.borrow().values().cloned());
        imports.sort_by(|left, right| left.path.cmp(&right.path));
        imports.dedup();
        imports
    }

    /// Registers a runtime or standard library used by this generated file.
    pub(crate) fn require(&self, name: &str, path: &str) {
        self.runtime.borrow_mut().insert(
            path.into(),
            GoImport {
                name: name.into(),
                path: path.into(),
            },
        );
    }

    /// A reusable IDL primitive supplied by the Go runtime.
    pub(crate) fn runtime_type(&self, name: &str) -> String {
        self.require("xidlgo", "github.com/xidl/xidl/golang/xidl-go");
        format!("xidlgo.{name}")
    }

    /// Directory an IDL path lives in, when it has one.
    fn package_path(unit: &str) -> Option<String> {
        let (dir, _) = unit.rsplit_once('/')?;
        let dir = dir.trim_matches('/');
        if dir.is_empty() {
            return None;
        }
        Some(dir.to_string())
    }

    /// Directory of the importing unit.
    fn package_path_of(unit: Option<&str>) -> Option<&str> {
        let unit = unit?;
        let (dir, _) = unit.rsplit_once('/')?;
        Some(dir.trim_matches('/'))
    }
}
