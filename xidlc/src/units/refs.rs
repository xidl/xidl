//! Cross-unit type references written in a unit's source.

use super::error::ImportError;
use similar::TextDiff;
use std::borrow::Cow;
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
        if bindings.is_empty() {
            return Ok(Self {
                source: source.to_string(),
                refs: Vec::new(),
            });
        }
        let mut normalized = NormalizedSource::new(source);
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_idl::language())
            .map_err(|err| ImportError::Parse {
                path: std::path::PathBuf::new(),
                reason: format!("failed to load the IDL grammar: {err}"),
            })?;
        let tree = parser
            .parse(normalized.as_str(), None)
            .ok_or_else(|| ImportError::Parse {
                path: std::path::PathBuf::new(),
                reason: "failed to parse the unit source".to_string(),
            })?;
        if tree.root_node().has_error() {
            return Err(ImportError::Parse {
                path: std::path::PathBuf::new(),
                reason: "failed to parse normalized unit references".to_string(),
            });
        }

        let mut rewrite = RefRewrite::new(bindings, &mut normalized);
        rewrite.walk(tree.root_node())?;
        let (refs, mut inserts) = rewrite.into_parts();

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
}

/// A source string normalized for tree-sitter plus a map back to raw offsets.
struct NormalizedSource<'a> {
    original: &'a str,
    normalized: Cow<'a, str>,
    offsets: Option<Vec<usize>>,
}

impl<'a> NormalizedSource<'a> {
    fn new(original: &'a str) -> Self {
        let normalized = xidl_parser::parser::normalize_source_for_tree_sitter(original);
        Self {
            original,
            normalized,
            offsets: None,
        }
    }

    fn as_str(&self) -> &str {
        self.normalized.as_ref()
    }

    fn original_offset(&mut self, offset: usize) -> Result<usize, ImportError> {
        if self.normalized.len() == self.original.len() {
            return Ok(offset);
        }
        if self.offsets.is_none() {
            self.offsets = Some(Self::diff_offsets(self.original, self.normalized.as_ref()));
        }
        let offsets = self.offsets.as_ref().ok_or_else(|| ImportError::Parse {
            path: std::path::PathBuf::new(),
            reason: "failed to build the normalized-reference offset map".to_string(),
        })?;
        let original = offsets
            .get(offset)
            .copied()
            .ok_or_else(|| ImportError::Parse {
                path: std::path::PathBuf::new(),
                reason: format!(
                    "normalized reference offset {offset} exceeds {} bytes",
                    self.normalized.len()
                ),
            })?;
        if !self.original.is_char_boundary(original) {
            return Err(ImportError::Parse {
                path: std::path::PathBuf::new(),
                reason: format!(
                    "normalized reference offset {offset} maps inside a UTF-8 character"
                ),
            });
        }
        Ok(original)
    }

    fn diff_offsets(original: &str, normalized: &str) -> Vec<usize> {
        let mut offsets = vec![0; normalized.len() + 1];
        let mut original_byte = 0;
        let mut normalized_byte = 0;
        for change in TextDiff::from_chars(original, normalized).iter_all_changes() {
            let len = change.value().len();
            match change.tag() {
                similar::ChangeTag::Equal => {
                    for index in 0..len {
                        offsets[normalized_byte + index] = original_byte + index;
                    }
                    original_byte += len;
                    normalized_byte += len;
                }
                similar::ChangeTag::Delete => original_byte += len,
                similar::ChangeTag::Insert => {
                    for index in 0..len {
                        offsets[normalized_byte + index] = original_byte;
                    }
                    normalized_byte += len;
                }
            }
        }
        offsets[normalized.len()] = original.len();
        offsets
    }
}

/// Walk state for cross-unit references and qualified-version insertions.
struct RefRewrite<'a, 'b> {
    bindings: &'b BTreeMap<String, Option<String>>,
    normalized: &'b mut NormalizedSource<'a>,
    refs: Vec<UnitRef>,
    inserts: Vec<(usize, String)>,
}

impl<'a, 'b> RefRewrite<'a, 'b> {
    fn new(
        bindings: &'b BTreeMap<String, Option<String>>,
        normalized: &'b mut NormalizedSource<'a>,
    ) -> Self {
        Self {
            bindings,
            normalized,
            refs: Vec::new(),
            inserts: Vec::new(),
        }
    }

    fn into_parts(self) -> (Vec<UnitRef>, Vec<(usize, String)>) {
        (self.refs, self.inserts)
    }

    /// Walks `node`, recording references and version insertions.
    fn walk(&mut self, node: Node<'_>) -> Result<(), ImportError> {
        if node.kind() == "scoped_name" {
            self.visit(node)?;
            return Ok(());
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk(child)?;
        }
        Ok(())
    }

    /// Records one outermost `scoped_name` when an import names its prefix.
    fn visit(&mut self, node: Node<'_>) -> Result<(), ImportError> {
        if node.parent().map(|parent| parent.kind()) == Some("scoped_name") {
            return Ok(());
        }
        let text = node
            .utf8_text(self.normalized.as_str().as_bytes())
            .map_err(|err| ImportError::Parse {
                path: std::path::PathBuf::new(),
                reason: format!("reference is not valid utf-8: {err}"),
            })?
            .to_string();
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
        let Some(bound) = self.bindings.get(prefix) else {
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
            self.refs.push(UnitRef {
                prefix: prefix.to_string(),
                version: bound.clone(),
                name: tail,
            });
            if let Some(bound) = bound {
                let offset = self
                    .normalized
                    .original_offset(node.start_byte() + separator)?;
                self.inserts.push((offset, format!("::{bound}")));
            }
            return Ok(());
        }
        let version = tail[0].clone();
        self.refs.push(UnitRef {
            prefix: prefix.to_string(),
            version: Some(version),
            name: tail[1..].to_vec(),
        });
        Ok(())
    }
}
