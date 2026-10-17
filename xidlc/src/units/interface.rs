//! The declarations one unit exposes to the units that import it.

use super::error::ImportError;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use xidl_parser::hir;
use xidl_parser::versioning::VersioningRecord;

/// What one compilation unit exposes to its importers.
///
/// A versioned unit exposes one set of names per API version, which is what a
/// reference such as `timestamp::v1::Instant` is checked against. An
/// unversioned unit exposes one set for every reference.
#[derive(Debug, Clone)]
pub struct UnitInterface {
    per_version: BTreeMap<String, BTreeSet<String>>,
    aliases: BTreeMap<String, String>,
    unversioned: Option<BTreeSet<String>>,
}

impl UnitInterface {
    /// Reads the interface `source` exposes.
    ///
    /// # Errors
    ///
    /// Returns [`ImportError::Parse`] when the unit cannot be read as IDL, so a
    /// broken dependency is reported against the file that holds it.
    pub(super) fn of(
        source: &str,
        path: &Path,
        mounts: &BTreeMap<String, PathBuf>,
    ) -> Result<Self, ImportError> {
        let spec = Self::lower(source, path, mounts)?;
        let record = VersioningRecord::analyze(&spec).map_err(|err| Self::error(path, err))?;
        let Some(record) = record else {
            return Ok(Self {
                per_version: BTreeMap::new(),
                aliases: BTreeMap::new(),
                unversioned: Some(Self::names(&spec.0)),
            });
        };
        let mut per_version = BTreeMap::new();
        let mut aliases = BTreeMap::new();
        for (id, definition) in record.timeline().iter() {
            let projected = record
                .project(&spec, id)
                .map_err(|err| Self::error(path, err))?;
            per_version.insert(definition.ident.clone(), Self::names(&projected.0));
            aliases.insert(definition.value.clone(), definition.ident.clone());
        }
        Ok(Self {
            per_version,
            aliases,
            unversioned: None,
        })
    }

    /// Whether the unit declares an API version timeline.
    pub(super) fn is_versioned(&self) -> bool {
        !self.per_version.is_empty()
    }

    /// Version idents the unit declares, in declaration order.
    pub(super) fn versions(&self) -> Vec<String> {
        self.per_version.keys().cloned().collect()
    }

    /// Whether `version` names a version of this unit, by ident or by value.
    pub(super) fn knows_version(&self, version: &str) -> bool {
        let ident = self
            .aliases
            .get(version)
            .map(String::as_str)
            .unwrap_or(version);
        self.per_version.contains_key(ident)
    }

    /// Whether `name` is declared at `version`.
    ///
    /// `version` may be a member ident or its display value.
    pub(super) fn declares(&self, version: Option<&str>, name: &str) -> bool {
        let Some(version) = version else {
            return self
                .unversioned
                .as_ref()
                .is_some_and(|names| names.contains(name));
        };
        let ident = self
            .aliases
            .get(version)
            .map(String::as_str)
            .unwrap_or(version);
        self.per_version
            .get(ident)
            .is_some_and(|names| names.contains(name))
    }

    /// Lowers one unit into the HIR its interface is read from.
    fn lower(
        source: &str,
        path: &Path,
        mounts: &BTreeMap<String, PathBuf>,
    ) -> Result<hir::Specification, ImportError> {
        let mut resolver = crate::library::include_resolver(mounts);
        let typed =
            xidl_parser::parser::parser_text_with_resolver(source, path.to_str(), &mut *resolver)
                .map_err(|err| Self::error(path, err))?;
        let mut properties = hir::ParserProperties::new();
        // The interface lists what the author wrote, without the RPC wrapper
        // declarations an expanded interface adds.
        properties.insert(
            "expand_interface".to_string(),
            serde_json::Value::Bool(false),
        );
        hir::Specification::from_typed_ast_with_properties_and_path(typed, properties, path)
            .map_err(|err| Self::error(path, err))
    }

    /// Names every declaration of `definitions` introduces.
    fn names(definitions: &[hir::Definition]) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        for definition in definitions {
            match definition {
                hir::Definition::ModuleDcl(module) => {
                    names.insert(module.ident.clone());
                    names.extend(Self::names(&module.definition));
                }
                hir::Definition::TypeDcl(type_dcl) => Self::type_dcl_names(type_dcl, &mut names),
                hir::Definition::ConstrTypeDcl(constr) => {
                    names.insert(Self::constr_ident(constr).to_string());
                }
                hir::Definition::ConstDcl(const_dcl) => {
                    names.insert(const_dcl.ident.clone());
                }
                hir::Definition::ExceptDcl(except) => {
                    names.insert(except.ident.clone());
                }
                hir::Definition::InterfaceDcl(interface) => {
                    Self::interface_names(interface, &mut names)
                }
                hir::Definition::Pragma(_) => {}
            }
        }
        names
    }

    /// Names a `type_dcl` introduces.
    fn type_dcl_names(type_dcl: &hir::TypeDcl, names: &mut BTreeSet<String>) {
        match type_dcl {
            hir::TypeDcl::ConstrTypeDcl(constr) => {
                names.insert(Self::constr_ident(constr).to_string());
            }
            hir::TypeDcl::TypedefDcl(typedef) => {
                names.extend(typedef.decl.iter().map(|decl| match decl {
                    hir::Declarator::SimpleDeclarator(simple) => simple.0.clone(),
                    hir::Declarator::ArrayDeclarator(array) => array.ident.clone(),
                }));
            }
            hir::TypeDcl::NativeDcl(native) => {
                names.insert(native.decl.0.clone());
            }
        }
    }

    /// Name a constructed type declaration introduces.
    fn constr_ident(constr: &hir::ConstrTypeDcl) -> &str {
        match constr {
            hir::ConstrTypeDcl::StructDcl(value) => &value.ident,
            hir::ConstrTypeDcl::StructForwardDcl(value) => &value.ident,
            hir::ConstrTypeDcl::UnionDef(value) => &value.ident,
            hir::ConstrTypeDcl::UnionForwardDcl(value) => &value.ident,
            hir::ConstrTypeDcl::EnumDcl(value) => &value.ident,
            hir::ConstrTypeDcl::BitsetDcl(value) => &value.ident,
            hir::ConstrTypeDcl::BitmaskDcl(value) => &value.ident,
        }
    }

    /// Names an interface declares together with the types inside its body.
    fn interface_names(interface: &hir::InterfaceDcl, names: &mut BTreeSet<String>) {
        match &interface.decl {
            hir::InterfaceDclInner::InterfaceForwardDcl(forward) => {
                names.insert(forward.ident.clone());
            }
            hir::InterfaceDclInner::InterfaceDef(definition) => {
                names.insert(definition.header.ident.clone());
                let Some(body) = &definition.interface_body else {
                    return;
                };
                for export in &body.0 {
                    match export {
                        hir::Export::TypeDcl(type_dcl) => Self::type_dcl_names(type_dcl, names),
                        hir::Export::ConstDcl(const_dcl) => {
                            names.insert(const_dcl.ident.clone());
                        }
                        hir::Export::ExceptDcl(except) => {
                            names.insert(except.ident.clone());
                        }
                        hir::Export::OpDcl(_) | hir::Export::AttrDcl(_) => {}
                    }
                }
            }
        }
    }

    /// Names the file a failed interface read came from.
    fn error(path: &Path, reason: impl std::fmt::Display) -> ImportError {
        ImportError::Parse {
            path: path.to_path_buf(),
            reason: reason.to_string(),
        }
    }
}
