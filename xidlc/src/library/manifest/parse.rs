use super::{Dependency, Manifest, ManifestError, Revision};
use semver::{Version, VersionReq};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// Manifest fields as they appear on disk, before validation.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    name: String,
    version: String,
    #[serde(default)]
    files: Option<Vec<String>>,
    #[serde(default)]
    exclude: Vec<String>,
    #[serde(default)]
    compiler: Option<String>,
    #[serde(default)]
    dependencies: BTreeMap<String, RawDependency>,
}

/// One `[dependencies]` entry as it appears on disk, before validation.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDependency {
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    rev: Option<String>,
    #[serde(default)]
    tag: Option<String>,
}

/// Rejects `field` when its value is blank.
fn required_text(path: &Path, field: &str, value: &str) -> Result<(), ManifestError> {
    if value.trim().is_empty() {
        return Err(ManifestError::InvalidField {
            path: path.to_path_buf(),
            field: field.to_string(),
            reason: "must not be empty".to_string(),
        });
    }
    Ok(())
}

/// Parses `value` as a semantic version, naming `field` on failure.
fn parse_version(path: &Path, field: &str, value: &str) -> Result<Version, ManifestError> {
    Version::parse(value).map_err(|err| ManifestError::InvalidField {
        path: path.to_path_buf(),
        field: field.to_string(),
        reason: format!("`{value}` is not a semantic version: {err}"),
    })
}

/// Parses `value` as a compiler version requirement, naming `field` on failure.
fn parse_compiler(path: &Path, value: &str) -> Result<VersionReq, ManifestError> {
    VersionReq::parse(value).map_err(|err| ManifestError::InvalidField {
        path: path.to_path_buf(),
        field: "compiler".to_string(),
        reason: format!("`{value}` is not a semantic version requirement: {err}"),
    })
}

/// Validates one raw dependency entry into a [`Dependency`].
fn parse_dependency(
    path: &Path,
    name: &str,
    raw: RawDependency,
) -> Result<Dependency, ManifestError> {
    let base = format!("dependencies.{name}");
    let invalid = |field: &str, reason: &str| ManifestError::InvalidField {
        path: path.to_path_buf(),
        field: field.to_string(),
        reason: reason.to_string(),
    };

    match (raw.src, raw.path) {
        (Some(_), Some(_)) => Err(invalid(
            &base,
            "declares both `src` and `path`; keep exactly one",
        )),
        (Some(src), None) => {
            required_text(path, &format!("{base}.src"), &src)?;
            let revision = match (raw.rev, raw.tag) {
                (Some(rev), None) => {
                    required_text(path, &format!("{base}.rev"), &rev)?;
                    Revision::Rev(rev)
                }
                (None, Some(tag)) => {
                    required_text(path, &format!("{base}.tag"), &tag)?;
                    Revision::Tag(tag)
                }
                (Some(_), Some(_)) => {
                    return Err(invalid(
                        &base,
                        "declares both `rev` and `tag`; keep exactly one",
                    ));
                }
                (None, None) => {
                    return Err(invalid(
                        &base,
                        "needs either `rev` or `tag` to pin a revision",
                    ));
                }
            };
            Ok(Dependency::new(src, revision))
        }
        (None, Some(path_dependency)) => {
            required_text(path, &format!("{base}.path"), &path_dependency)?;
            if raw.rev.is_some() || raw.tag.is_some() {
                return Err(invalid(
                    &base,
                    "`path` dependencies cannot declare `rev` or `tag`",
                ));
            }
            Ok(Dependency::from_path(path_dependency))
        }
        (None, None) => Err(invalid(&base, "needs exactly one of `src` or `path`")),
    }
}

/// Parses and validates manifest text into a [`Manifest`].
pub(super) fn from_str(path: &Path, text: &str) -> Result<Manifest, ManifestError> {
    let raw: RawManifest = toml::from_str(text).map_err(|source| ManifestError::Deserialize {
        path: path.to_path_buf(),
        source,
    })?;

    required_text(path, "name", &raw.name)?;
    let version = parse_version(path, "version", &raw.version)?;
    let compiler = raw
        .compiler
        .as_deref()
        .map(|value| parse_compiler(path, value))
        .transpose()?;

    let mut dependencies = BTreeMap::new();
    for (name, raw_dependency) in raw.dependencies {
        required_text(path, &format!("dependencies.{name}"), &name)?;
        dependencies.insert(name.clone(), parse_dependency(path, &name, raw_dependency)?);
    }

    Ok(Manifest {
        name: raw.name,
        version,
        files: raw.files,
        exclude: raw.exclude,
        compiler,
        dependencies,
    })
}
