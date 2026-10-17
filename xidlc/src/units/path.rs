/// An `import` path split into its target and its trailing version selector.
///
/// A path of the form `"pkg/util.idl@v1"` names the file `pkg/util.idl` and
/// binds `v1` as the default API version of every reference to that unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPath {
    target: String,
    version: Option<String>,
}

impl ImportPath {
    /// Splits `raw` at its trailing `@<version>` selector.
    ///
    /// A leading `@` belongs to the path, so `"@xidl/http"` keeps its name.
    pub fn parse(raw: &str) -> Self {
        let split = raw.rfind('@').filter(|index| *index > 0);
        let Some(index) = split else {
            return Self {
                target: raw.to_string(),
                version: None,
            };
        };
        let (target, selector) = raw.split_at(index);
        let selector = selector.trim_start_matches('@');
        if selector.is_empty() {
            return Self {
                target: raw.to_string(),
                version: None,
            };
        }
        Self {
            target: target.to_string(),
            version: Some(selector.to_string()),
        }
    }

    /// The path part, without any version selector.
    pub fn target(&self) -> &str {
        &self.target
    }

    /// The API version this import binds, when it carries a selector.
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// The reference prefix: the last path segment without its `.idl` suffix.
    ///
    /// Returns `None` for paths without a usable name, such as `"/"` or `".."`.
    pub fn prefix(&self) -> Option<String> {
        let trimmed = self.target.trim_end_matches('/');
        let last = trimmed.rsplit('/').next()?;
        let name = last.strip_suffix(".idl").unwrap_or(last);
        if name.is_empty() || name == "." || name == ".." {
            return None;
        }
        Some(name.to_string())
    }
}
