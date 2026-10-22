use super::{DependencySource, Manifest, ManifestError, Revision};
use semver::{Version, VersionReq};
use std::error::Error;
use std::path::Path;

/// Parses `text`, expecting success.
fn parse(text: &str) -> Manifest {
    Manifest::parse(Path::new("xidl.toml"), text).expect("manifest should parse")
}

/// Parses `text`, expecting a validation failure.
fn parse_err(text: &str) -> ManifestError {
    Manifest::parse(Path::new("xidl.toml"), text).expect_err("manifest should be rejected")
}

/// Reports the dotted field named by an [`ManifestError::InvalidField`].
fn invalid_field(error: &ManifestError) -> &str {
    match error {
        ManifestError::InvalidField { field, .. } => field,
        other => panic!("expected an invalid field error, got {other}"),
    }
}

#[test]
fn parses_full_manifest() {
    let manifest = parse(
        r#"
name = "timestamp"
version = "1.2.3"
files = ["*.idl", "lib/*.idl"]
exclude = ["draft.idl"]
compiler = ">=0.100"

[dependencies]
"timestamp" = { src = "github.com/x/timestamp", tag = "v1.2.3" }
"other" = { src = "https://github.com/x/other", rev = "9f0dabc" }
"#,
    );

    assert_eq!(manifest.name(), "timestamp");
    assert_eq!(manifest.version(), &Version::new(1, 2, 3));
    assert_eq!(
        manifest.files(),
        Some(["*.idl".to_string(), "lib/*.idl".to_string()].as_slice())
    );
    assert_eq!(manifest.exclude().to_vec(), vec!["draft.idl".to_string()]);
    assert_eq!(
        manifest.compiler(),
        Some(&VersionReq::parse(">=0.100").unwrap())
    );

    let timestamp = &manifest.dependencies()["timestamp"];
    assert_eq!(timestamp.src(), Some("github.com/x/timestamp"));
    assert_eq!(
        timestamp.revision(),
        Some(&Revision::Tag("v1.2.3".to_string()))
    );
    assert_eq!(timestamp.path(), None);
    assert_eq!(
        timestamp
            .git()
            .map(|(src, revision)| (src, revision.as_str())),
        Some(("github.com/x/timestamp", "v1.2.3"))
    );

    let other = &manifest.dependencies()["other"];
    assert_eq!(other.src(), Some("https://github.com/x/other"));
    assert_eq!(
        other.revision(),
        Some(&Revision::Rev("9f0dabc".to_string()))
    );
}

#[test]
fn parses_minimal_manifest() {
    let manifest = parse("name = \"timestamp\"\nversion = \"0.1.0\"\n");

    assert_eq!(manifest.files(), None);
    assert!(manifest.exclude().is_empty());
    assert!(manifest.compiler().is_none());
    assert!(manifest.dependencies().is_empty());
}

#[test]
fn parses_path_dependency() {
    let manifest = parse(
        "name = \"project\"\nversion = \"1.0.0\"\n\n[dependencies]\n\
         \"timestamp\" = { path = \"../timestamp\" }\n",
    );

    let timestamp = &manifest.dependencies()["timestamp"];
    assert_eq!(timestamp.path(), Some("../timestamp"));
    assert_eq!(
        timestamp.source(),
        &DependencySource::Path {
            path: "../timestamp".to_string(),
        }
    );
    assert!(timestamp.src().is_none());
    assert!(timestamp.revision().is_none());
    assert!(timestamp.git().is_none());
}

#[test]
fn rejects_unknown_top_level_field() {
    let error = parse_err("name = \"a\"\nversion = \"1.0.0\"\nnamespace = \"A\"\n");
    assert!(matches!(error, ManifestError::Deserialize { .. }));
    assert!(error.to_string().contains("namespace"));
}

#[test]
fn rejects_missing_name() {
    let error = parse_err("version = \"1.0.0\"\n");
    assert!(matches!(error, ManifestError::Deserialize { .. }));
}

#[test]
fn rejects_non_semver_version() {
    let error = parse_err("name = \"a\"\nversion = \"one\"\n");
    assert_eq!(invalid_field(&error), "version");
}

#[test]
fn rejects_invalid_compiler_range() {
    let error = parse_err("name = \"a\"\nversion = \"1.0.0\"\ncompiler = \"not a range\"\n");
    assert_eq!(invalid_field(&error), "compiler");
}

#[test]
fn rejects_dependency_without_revision() {
    let error = parse_err(
        "name = \"a\"\nversion = \"1.0.0\"\n\n[dependencies]\n\"b\" = { src = \"github.com/x/b\" }\n",
    );
    assert_eq!(invalid_field(&error), "dependencies.b");
}

#[test]
fn rejects_dependency_with_both_source_and_path() {
    let error = parse_err(
        "name = \"a\"\nversion = \"1.0.0\"\n\n[dependencies]\n\
         \"b\" = { src = \"github.com/x/b\", path = \"../b\", tag = \"v1\" }\n",
    );
    assert_eq!(invalid_field(&error), "dependencies.b");
    assert!(
        error.to_string().contains("both `src` and `path`"),
        "{error}"
    );
}

#[test]
fn rejects_path_dependency_with_revision() {
    let error = parse_err(
        "name = \"a\"\nversion = \"1.0.0\"\n\n[dependencies]\n\
         \"b\" = { path = \"../b\", rev = \"abc\" }\n",
    );
    assert_eq!(invalid_field(&error), "dependencies.b");
    assert!(
        error.to_string().contains("cannot declare `rev` or `tag`"),
        "{error}"
    );
}

#[test]
fn rejects_blank_path_dependency() {
    let error = parse_err(
        "name = \"a\"\nversion = \"1.0.0\"\n\n[dependencies]\n\
         \"b\" = { path = \"  \" }\n",
    );
    assert_eq!(invalid_field(&error), "dependencies.b.path");
}

#[test]
fn rejects_dependency_without_source_or_path() {
    let error = parse_err("name = \"a\"\nversion = \"1.0.0\"\n\n[dependencies]\n\"b\" = { }\n");
    assert_eq!(invalid_field(&error), "dependencies.b");
    assert!(
        error.to_string().contains("exactly one of `src` or `path`"),
        "{error}"
    );
}

#[test]
fn rejects_dependency_with_both_revisions() {
    let error = parse_err(
        "name = \"a\"\nversion = \"1.0.0\"\n\n[dependencies]\n\
         \"b\" = { src = \"github.com/x/b\", rev = \"abc\", tag = \"v1\" }\n",
    );
    assert_eq!(invalid_field(&error), "dependencies.b");
}

#[test]
fn rejects_unknown_dependency_field() {
    let error = parse_err(
        "name = \"a\"\nversion = \"1.0.0\"\n\n[dependencies]\n\
         \"b\" = { src = \"github.com/x/b\", tag = \"v1\", version = \"1\" }\n",
    );
    assert!(matches!(error, ManifestError::Deserialize { .. }));
}

#[test]
fn rejects_blank_dependency_src() {
    let error = parse_err(
        "name = \"a\"\nversion = \"1.0.0\"\n\n[dependencies]\n\
         \"b\" = { src = \"  \", tag = \"v1\" }\n",
    );
    assert_eq!(invalid_field(&error), "dependencies.b.src");
}

#[test]
fn accepts_compiler_honours_the_declared_range() {
    let manifest = parse("name = \"a\"\nversion = \"1.0.0\"\ncompiler = \">=0.100\"\n");

    assert!(manifest.accepts_compiler(&Version::new(0, 100, 0)));
    assert!(manifest.accepts_compiler(&Version::new(1, 0, 0)));
    assert!(!manifest.accepts_compiler(&Version::new(0, 99, 9)));
}

#[test]
fn accepts_compiler_without_a_range() {
    let manifest = parse("name = \"a\"\nversion = \"1.0.0\"\n");
    assert!(manifest.accepts_compiler(&Version::new(0, 0, 1)));
}

#[test]
fn load_reports_a_missing_manifest() {
    let error = Manifest::load(Path::new("/nonexistent/xidl.toml"))
        .expect_err("load should fail for a missing file");
    assert!(matches!(error, ManifestError::Read { .. }));
}

#[test]
fn revision_reports_its_text() {
    assert_eq!(Revision::Tag("v1".to_string()).as_str(), "v1");
    assert_eq!(Revision::Rev("abc".to_string()).as_str(), "abc");
}

#[test]
fn read_errors_name_the_manifest() {
    let error = Manifest::load(Path::new("/nonexistent/xidl.toml"))
        .expect_err("missing manifests cannot load");

    assert!(
        error.to_string().contains("failed to read manifest"),
        "{error}"
    );
    assert!(error.source().is_some());
}

#[test]
fn deserialize_errors_name_the_manifest() {
    let error = parse_err("name = \"a\"\nversion = \"1.0.0\"\nnamespace = \"A\"\n");

    assert!(
        error.to_string().contains("failed to parse manifest"),
        "{error}"
    );
    assert!(error.source().is_some());
}

#[test]
fn field_errors_name_the_field_and_carry_no_source() {
    let error = parse_err("name = \"a\"\nversion = \"one\"\n");

    assert!(
        error
            .to_string()
            .contains("invalid `version` in manifest xidl.toml"),
        "{error}"
    );
    assert!(error.source().is_none());
}
