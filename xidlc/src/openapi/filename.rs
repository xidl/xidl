/// The shared name of a generated OpenAPI artifact and its external references.
pub(crate) struct OpenApiFilename;

impl OpenApiFilename {
    const PREFIX: &str = "openapi_";
    const SUFFIX: &str = ".json";

    /// Names the document for a unit and an optional declared version member.
    #[cfg(feature = "gen-openapi")]
    pub(crate) fn render(unit: &str, version: Option<&str>) -> String {
        let prefix = Self::PREFIX;
        let suffix = Self::SUFFIX;
        match version {
            Some(version) => format!("{prefix}{unit}.{version}{suffix}"),
            None => format!("{prefix}{unit}{suffix}"),
        }
    }

    /// Decodes a generated filename against a known unit prefix.
    #[cfg(feature = "cli")]
    pub(crate) fn version_for<'a>(file: &'a str, unit: &str) -> Option<Option<&'a str>> {
        let tail = file
            .strip_prefix(Self::PREFIX)?
            .strip_suffix(Self::SUFFIX)?
            .strip_prefix(unit)?;
        if tail.is_empty() {
            return Some(None);
        }
        tail.strip_prefix('.')
            .filter(|version| !version.is_empty() && !version.contains('.'))
            .map(Some)
    }
}
