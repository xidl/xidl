use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use xidlc::driver::{ArgsGenerate, Driver};

/// An isolated editable copy of the split address libraries.
pub struct Libraries {
    root: TempDir,
}

impl Libraries {
    /// Copies real business IDL without modifying the shared BDD fixtures.
    pub fn new() -> Self {
        let root = tempfile::tempdir().expect("library fixture directory");
        Self::copy_tree(&Self::data().join("versioned_libraries"), root.path());
        Self { root }
    }

    /// Resolves a path inside the editable fixture.
    pub fn path(&self, relative: &str) -> PathBuf {
        self.root.path().join(relative)
    }

    /// The original single-file contract used by the owning snapshots.
    pub fn flat_contract() -> PathBuf {
        Self::data().join("versioned_inheritance.idl")
    }

    /// Generates from a manifest or raw IDL through the complete CLI driver.
    pub fn generate(
        &self,
        target: &str,
        input: &Path,
        version: Option<&str>,
    ) -> BTreeMap<String, String> {
        let output = tempfile::tempdir().expect("generated library output");
        Self::try_generate(target, input, version, output.path())
            .unwrap_or_else(|error| panic!("{target} {}: {error}", input.display()));
        fs::read_dir(output.path())
            .expect("generated library files")
            .map(|entry| {
                let entry = entry.expect("generated library entry");
                (
                    entry.file_name().to_string_lossy().into_owned(),
                    fs::read_to_string(entry.path()).expect("generated library source"),
                )
            })
            .collect()
    }

    /// Runs generation into caller-owned output to inspect failure atomicity.
    pub fn generate_error(&self, target: &str, input: &Path, output: &Path) -> String {
        Self::try_generate(target, input, None, output)
            .expect_err("invalid library must fail generation")
    }

    fn try_generate(
        target: &str,
        input: &Path,
        version: Option<&str>,
        output: &Path,
    ) -> Result<(), String> {
        Driver::run(ArgsGenerate {
            lang: target.to_string(),
            out_dir: output.to_string_lossy().into_owned(),
            client: !target.ends_with("adapters"),
            server: !target.ends_with("adapters"),
            dry_run: false,
            mock: false,
            api_version: version.map(str::to_string),
            api_versions: None,
            openapi_version: None,
            files: vec![input.to_path_buf()],
        })
        .map_err(|error| error.to_string())
    }

    fn data() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../bdd/features/data")
    }

    fn copy_tree(source: &Path, target: &Path) {
        fs::create_dir_all(target).expect("create fixture subdirectory");
        for entry in fs::read_dir(source).expect("library source directory") {
            let entry = entry.expect("library source entry");
            let destination = target.join(entry.file_name());
            if entry.file_type().expect("library entry type").is_dir() {
                Self::copy_tree(&entry.path(), &destination);
            } else {
                fs::copy(entry.path(), destination).expect("copy library fixture");
            }
        }
    }
}
