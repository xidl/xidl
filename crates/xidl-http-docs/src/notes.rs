use crate::error::{DocumentationError, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReleaseNote {
    pub unit: String,
    pub from: String,
    pub to: String,
    pub title: String,
    #[serde(default)]
    pub reviewed: bool,
    #[serde(default)]
    pub impact: String,
    #[serde(default)]
    pub upgrade: String,
    #[serde(skip)]
    pub markdown: String,
}

impl ReleaseNote {
    pub fn read(path: &Path) -> Result<Self> {
        let source = std::fs::read_to_string(path)?;
        let source = source.replace("\r\n", "\n");
        let source = source.strip_prefix("+++\n").ok_or_else(|| {
            DocumentationError::Contract(format!(
                "release note '{}' requires +++ TOML front matter",
                path.display()
            ))
        })?;
        let (header, body) = source.split_once("\n+++\n").ok_or_else(|| {
            DocumentationError::Contract(format!(
                "release note '{}' has no closing +++ delimiter",
                path.display()
            ))
        })?;
        let mut note: Self = toml::from_str(header)?;
        note.markdown = body.trim().into();
        if note.title.trim().is_empty() {
            return Err(DocumentationError::Contract(format!(
                "release note '{}' has no title",
                path.display()
            )));
        }
        Ok(note)
    }

    pub fn ready(&self) -> bool {
        self.reviewed && !self.impact.trim().is_empty() && !self.upgrade.trim().is_empty()
    }
}
