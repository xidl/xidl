//! Cross-unit type references written in a unit's source.

use super::error::ImportError;
use std::collections::BTreeMap;
use tree_sitter::{Node, Parser};

/// One reference a unit makes into another compilation unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitRef {
    prefix: String,
    version: Option<String>,
    name: Vec<String>,
}

impl UnitRef {
    /// Reference prefix, such as `timestamp`.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// API version the reference resolves to.
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// Declared name the reference names, without the prefix or version.
    pub fn name(&self) -> &str {
        self.name.first().map(String::as_str).unwrap_or_default()
    }

    /// Reference as written in the target language, such as `v1::Instant`.
    pub fn path(&self) -> String {
        self.name.join("::")
    }

    /// Full reference, such as `timestamp::v1::Instant`.
    pub fn qualified(&self) -> String {
        let mut parts = vec![self.prefix.clone()];
        parts.extend(self.version.clone());
        parts.extend(self.name.iter().cloned());
        parts.join("::")
    }
}

/// A unit source together with the cross-unit references it makes.
#[derive(Debug, Clone)]
pub(super) struct PreparedSource {
    source: String,
    refs: Vec<UnitRef>,
}

impl PreparedSource {
    /// Resolves every reference into an imported unit.
    ///
    /// `bindings` maps every import prefix to the API version that import
    /// bound. A reference that omits a bound version is qualified with it, so
    /// `import "timestamp@v1"` turns `timestamp::Instant` into
    /// `timestamp::v1::Instant`. References into names that no import binds are
    /// left untouched: they may name a declared module.
    ///
    /// # Errors
    ///
    /// Returns [`ImportError::Parse`] when the source cannot be read as IDL.
    pub(super) fn of(
        source: &str,
        bindings: &BTreeMap<String, Option<String>>,
    ) -> Result<Self, ImportError> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_idl::language())
            .map_err(|err| ImportError::Parse {
                path: std::path::PathBuf::new(),
                reason: format!("failed to load the IDL grammar: {err}"),
            })?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| ImportError::Parse {
                path: std::path::PathBuf::new(),
                reason: "failed to parse the unit source".to_string(),
            })?;

        let mut refs = Vec::new();
        let mut inserts = Vec::new();
        Self::walk(
            tree.root_node(),
            source.as_bytes(),
            bindings,
            &mut refs,
            &mut inserts,
        )?;

        let mut qualified = source.to_string();
        inserts.sort_by_key(|(offset, _)| std::cmp::Reverse(*offset));
        for (offset, text) in inserts {
            qualified.insert_str(offset, &text);
        }
        Ok(Self {
            source: qualified,
            refs,
        })
    }

    /// The source with every bound version spelled out.
    pub(super) fn source(&self) -> &str {
        &self.source
    }

    /// The references the unit makes.
    pub(super) fn refs(&self) -> &[UnitRef] {
        &self.refs
    }

    /// Walks `node`, recording references and version insertions.
    fn walk(
        node: Node<'_>,
        source: &[u8],
        bindings: &BTreeMap<String, Option<String>>,
        refs: &mut Vec<UnitRef>,
        inserts: &mut Vec<(usize, String)>,
    ) -> Result<(), ImportError> {
        if node.kind() == "scoped_name" {
            Self::visit(node, source, bindings, refs, inserts)?;
            return Ok(());
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::walk(child, source, bindings, refs, inserts)?;
        }
        Ok(())
    }

    /// Records one outermost `scoped_name` when an import names its prefix.
    fn visit(
        node: Node<'_>,
        source: &[u8],
        bindings: &BTreeMap<String, Option<String>>,
        refs: &mut Vec<UnitRef>,
        inserts: &mut Vec<(usize, String)>,
    ) -> Result<(), ImportError> {
        if node.parent().map(|parent| parent.kind()) == Some("scoped_name") {
            return Ok(());
        }
        let text = node.utf8_text(source).map_err(|err| ImportError::Parse {
            path: std::path::PathBuf::new(),
            reason: format!("reference is not valid utf-8: {err}"),
        })?;
        if text.trim_start().starts_with("::") {
            return Ok(());
        }
        let Some(separator) = text.find("::") else {
            return Ok(());
        };
        let prefix = text[..separator].trim();
        if prefix.is_empty() {
            return Ok(());
        }
        let Some(bound) = bindings.get(prefix) else {
            return Ok(());
        };
        let rest = text[separator + 2..].trim();
        let mut parts = rest
            .split("::")
            .map(str::trim)
            .filter(|part| !part.is_empty());
        let Some(head) = parts.next() else {
            return Ok(());
        };
        let tail: Vec<String> = std::iter::once(head.to_string())
            .chain(parts.map(str::to_string))
            .collect();
        if tail.len() == 1 {
            refs.push(UnitRef {
                prefix: prefix.to_string(),
                version: bound.clone(),
                name: tail,
            });
            if let Some(bound) = bound {
                inserts.push((node.start_byte() + separator, format!("::{bound}")));
            }
            return Ok(());
        }
        let version = tail[0].clone();
        refs.push(UnitRef {
            prefix: prefix.to_string(),
            version: Some(version),
            name: tail[1..].to_vec(),
        });
        Ok(())
    }
}
