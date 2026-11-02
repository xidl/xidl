use crate::error::{DocumentationError, Result};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

/// Publication settings loaded independently of XIDL language annotations.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DocumentationConfig {
    /// Site title; otherwise taken from the HTTP package metadata.
    pub title: Option<String>,
    /// Deployment URL or absolute URL path, including any hosting subdirectory.
    pub base_url: String,
    /// Qualified operation or type identities to exclude from all output.
    pub exclude: BTreeSet<String>,
    /// Markdown release note files, relative to the configuration file.
    pub notes: Vec<PathBuf>,
    /// Reject changes without reviewed author impact and upgrade instructions.
    pub strict: bool,
    /// Maximum full-index bytes; exceeding the limit fails without truncation.
    pub full_max_bytes: usize,
    /// Maximum root-index bytes; per-version indexes remain independently readable.
    pub index_max_bytes: usize,
}

impl Default for DocumentationConfig {
    fn default() -> Self {
        Self {
            title: None,
            base_url: "/".into(),
            exclude: BTreeSet::new(),
            notes: Vec::new(),
            strict: false,
            full_max_bytes: 16 * 1024 * 1024,
            index_max_bytes: 256 * 1024,
        }
    }
}

impl DocumentationConfig {
    /// Reads an optional TOML file and resolves note paths relative to it.
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let mut config: Self = match path {
            Some(path) => toml::from_str(&std::fs::read_to_string(path)?)?,
            None => Self::default(),
        };
        if let Some(parent) = path.and_then(Path::parent) {
            for note in &mut config.notes {
                *note = parent.join(&note);
            }
        }
        config.validate()?;
        Ok(config)
    }

    pub(crate) fn validate(&self) -> Result<()> {
        let base = &self.base_url;
        if !(base.starts_with('/') || base.starts_with("https://") || base.starts_with("http://"))
            || base.starts_with("//")
            || base.contains(['?', '#', '\n', '\r', '"', '<', '>', ' '])
        {
            return Err(DocumentationError::Contract(
                "base_url must be an HTTP URL or absolute URL path".into(),
            ));
        }
        if self.full_max_bytes == 0 || self.index_max_bytes == 0 {
            return Err(DocumentationError::Contract(
                "AI index byte limits must be positive".into(),
            ));
        }
        self.base()?;
        Ok(())
    }

    pub(crate) fn base(&self) -> Result<url::Url> {
        let deployment = self.url("");
        let base = if self.base_url.starts_with('/') {
            url::Url::parse("https://xidl.invalid/").and_then(|root| root.join(&deployment))
        } else {
            url::Url::parse(&deployment)
        };
        let base = base.map_err(|error| {
            DocumentationError::Contract(format!("invalid deployment base_url: {error}"))
        })?;
        if base.path().starts_with("//") {
            return Err(DocumentationError::Contract(
                "base_url deployment path must not start with '//'".into(),
            ));
        }
        Ok(base)
    }

    pub(crate) fn destination(
        &self,
        page: &str,
        destination: &str,
    ) -> Result<Option<(String, Option<String>)>> {
        let base = self.base()?;
        let url = base
            .join(page)
            .and_then(|page| page.join(destination))
            .map_err(|error| {
                DocumentationError::Contract(format!(
                    "invalid documentation link '{destination}': {error}"
                ))
            })?;
        if !["https", "http", "mailto"].contains(&url.scheme()) {
            return Err(DocumentationError::Contract(format!(
                "unsupported documentation link '{destination}'"
            )));
        }
        if url.origin() == base.origin() {
            let anchor = url
                .fragment()
                .map(|fragment| {
                    percent_encoding::percent_decode_str(fragment)
                        .decode_utf8()
                        .map(|value| value.into_owned())
                        .map_err(|error| {
                            DocumentationError::Contract(format!(
                                "invalid documentation anchor '{fragment}': {error}"
                            ))
                        })
                })
                .transpose()?;
            Ok(url
                .path()
                .strip_prefix(base.path())
                .map(|local| (local.into(), anchor)))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn url(&self, path: &str) -> String {
        format!("{}/{path}", self.base_url.trim_end_matches('/'))
    }

    /// Keeps browser navigation on the serving origin with the configured base path.
    pub(crate) fn browser_url(&self, path: &str) -> Result<String> {
        Ok(format!("{}{path}", self.base()?.path()))
    }

    pub(crate) fn excluded(&self, unit: &str, name: &str) -> bool {
        self.exclude.contains(name)
            || self.exclude.contains(&format!("{unit}::{name}"))
            || self.exclude.contains(unit)
            || self.exclude.iter().any(|prefix| {
                name.starts_with(&format!("{prefix}::"))
                    || format!("{unit}::{name}").starts_with(&format!("{prefix}::"))
            })
    }
}
