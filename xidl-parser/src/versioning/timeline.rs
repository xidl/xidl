use super::VersioningError;
use crate::error::ParserResult;
use crate::hir::{Annotation, Enumerator, ParserProperties, ScopedName};
use std::collections::BTreeMap;

/// Position of a version inside the declaration order of the `@version` enum.
///
/// Versions are compared by this index only; the display value is never parsed
/// or sorted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VersionId(usize);

impl VersionId {
    /// Zero-based position in the version enum's declaration order.
    pub fn index(self) -> usize {
        self.0
    }
}

/// One member of the API version enum.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VersionDef {
    /// Stable machine name of the version: the enum member ident.
    pub ident: String,
    /// Display value of the version: the `@rename("...")` argument, or the ident.
    pub value: String,
}

/// Ordered API version history declared by the `@version` enum.
#[derive(Debug, Clone)]
pub struct VersionTimeline {
    enum_path: Vec<String>,
    enum_ident: String,
    versions: Vec<VersionDef>,
}

impl VersionTimeline {
    pub(crate) fn from_members(
        enum_path: String,
        members: &[Enumerator],
    ) -> Result<Self, VersioningError> {
        if members.is_empty() {
            return Err(VersioningError::EmptyVersionEnum { ident: enum_path });
        }
        let enum_path = enum_path
            .split("::")
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let enum_ident = enum_path.last().cloned().unwrap_or_default();
        let mut versions = Vec::new();
        let mut values: BTreeMap<String, String> = BTreeMap::new();
        for member in members {
            let value = member_value(member);
            if let Some(first) = values.get(&value) {
                return Err(VersioningError::DuplicateVersionValue {
                    value,
                    first: first.clone(),
                    second: member.ident.clone(),
                });
            }
            values.insert(value.clone(), member.ident.clone());
            versions.push(VersionDef {
                ident: member.ident.clone(),
                value,
            });
        }
        Ok(Self {
            enum_path,
            enum_ident,
            versions,
        })
    }

    /// Ident of the enum that declares the version history.
    pub fn enum_ident(&self) -> &str {
        self.enum_ident.as_str()
    }

    /// Number of declared versions.
    pub fn len(&self) -> usize {
        self.versions.len()
    }

    /// Whether the timeline declares no versions.
    pub fn is_empty(&self) -> bool {
        self.versions.is_empty()
    }

    /// Version definition for a position in declaration order.
    pub fn get(&self, version: VersionId) -> Option<&VersionDef> {
        self.versions.get(version.0)
    }

    /// All versions in declaration order.
    pub fn iter(&self) -> impl Iterator<Item = (VersionId, &VersionDef)> {
        self.versions
            .iter()
            .enumerate()
            .map(|(index, def)| (VersionId(index), def))
    }

    /// Every declared version as a `, `-separated `ident (value)` list.
    pub fn declared(&self) -> String {
        self.versions
            .iter()
            .map(|def| format!("{} ({})", def.ident, def.value))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Resolves a version by member ident or display value.
    pub(crate) fn find(&self, raw: &str) -> Result<VersionId, VersioningError> {
        let matches = self
            .versions
            .iter()
            .enumerate()
            .filter(|(_, def)| def.ident == raw || def.value == raw)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [index] => Ok(VersionId(*index)),
            [] => Err(VersioningError::UnknownSelectedVersion {
                selection: raw.to_string(),
                declared: self.declared(),
            }),
            [first, second, ..] => Err(VersioningError::AmbiguousSelectedVersion {
                selection: raw.to_string(),
                first: self.versions[*first].ident.clone(),
                second: self.versions[*second].ident.clone(),
            }),
        }
    }

    /// Resolves an `Enum::member` reference against the timeline.
    ///
    /// Returns `None` when the reference does not name a declared version of
    /// this timeline.
    pub(crate) fn resolve(&self, reference: &ScopedName) -> Option<VersionId> {
        let (member, prefix) = reference.name.split_last()?;
        let qualified = prefix == self.enum_path.as_slice()
            || (prefix.len() == 1 && prefix[0] == self.enum_ident);
        if !qualified {
            return None;
        }
        let index = self.versions.iter().position(|def| &def.ident == member)?;
        Some(VersionId(index))
    }
}

fn member_value(member: &Enumerator) -> String {
    member
        .annotations
        .iter()
        .find_map(|annotation| match annotation {
            Annotation::Rename { name } => Some(name.clone()),
            _ => None,
        })
        .unwrap_or_else(|| member.ident.clone())
}

/// Versions requested by the build for one generation run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionSelection {
    /// Every version declared by the timeline, in declaration order.
    All,
    /// Explicit member idents or display values.
    Versions(Vec<String>),
}

impl VersionSelection {
    /// Reads `api_version` / `api_versions` from generation properties.
    ///
    /// Returns `None` when neither property is present.
    pub(crate) fn from_properties(
        properties: &ParserProperties,
    ) -> ParserResult<Option<VersionSelection>> {
        let single = property(properties, super::API_VERSION_PROPERTY);
        let multiple = property(properties, super::API_VERSIONS_PROPERTY);
        match (single, multiple) {
            (Some(_), Some(_)) => Err(VersioningError::ConflictingSelection {
                first: super::API_VERSION_PROPERTY.to_string(),
                second: super::API_VERSIONS_PROPERTY.to_string(),
            }
            .into()),
            (Some(raw), None) => Self::single(&raw).map(Some),
            (None, Some(raw)) => Self::multiple(&raw).map(Some),
            (None, None) => Ok(None),
        }
    }

    fn single(raw: &str) -> ParserResult<Self> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err(VersioningError::EmptySelection.into());
        }
        if raw == "*" {
            return Err(VersioningError::SelectionRequiresOneVersion {
                selection: raw.to_string(),
            }
            .into());
        }
        Ok(Self::Versions(vec![raw.to_string()]))
    }

    fn multiple(raw: &str) -> ParserResult<Self> {
        let items = raw
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>();
        if items.is_empty() {
            return Err(VersioningError::EmptySelection.into());
        }
        if items.contains(&"*") {
            return Ok(Self::All);
        }
        Ok(Self::Versions(
            items.into_iter().map(ToString::to_string).collect(),
        ))
    }

    /// Resolves the selection into timeline positions, sorted by declaration order.
    pub(crate) fn resolve(&self, timeline: &VersionTimeline) -> ParserResult<Vec<VersionId>> {
        match self {
            Self::All => Ok(timeline.iter().map(|(id, _)| id).collect()),
            Self::Versions(raw) => {
                let mut resolved = Vec::new();
                for item in raw {
                    let id = timeline.find(item)?;
                    if !resolved.contains(&id) {
                        resolved.push(id);
                    }
                }
                resolved.sort();
                Ok(resolved)
            }
        }
    }

    /// Renders the request for diagnostics.
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::All => "*".to_string(),
            Self::Versions(raw) => raw.join(", "),
        }
    }
}

fn property(properties: &ParserProperties, key: &str) -> Option<String> {
    properties
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
}
