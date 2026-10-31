//! Builder API that assembles and runs IDL generation for Cargo build scripts.

use crate::error::Error;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "bundle")]
use crate::bundle::BundleBackend;

#[cfg(not(feature = "bundle"))]
use crate::cli::CliBackend;

/// The generation request assembled by [`Builder`] and executed by one backend.
pub(crate) struct GenerateSpec {
    pub(crate) lang: String,
    pub(crate) out_dir: PathBuf,
    pub(crate) client: bool,
    pub(crate) server: bool,
    pub(crate) mock: bool,
    pub(crate) api_version: Option<String>,
    pub(crate) api_versions: Option<String>,
    pub(crate) openapi_version: Option<String>,
    pub(crate) inputs: Vec<PathBuf>,
}

/// Configures and runs IDL code generation from a Cargo build script.
///
/// `Builder` provides a small fluent interface around the `xidlc` compiler.
/// The default configuration targets Rust output, writes into `OUT_DIR`, and
/// enables both client and server generation.
///
/// # Example
///
/// ```no_run
/// fn main() -> Result<(), xidl_build::Error> {
///     xidl_build::Builder::new()
///         .with_lang("openapi")
///         .with_output_filename("api.json")
///         .compile(&["idl/petstore.idl"])?;
///
///     Ok(())
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Builder {
    lang: String,
    out_dir: Option<PathBuf>,
    output_filename: Option<PathBuf>,
    client: bool,
    server: bool,
    mock: bool,
    api_version: Option<String>,
    api_versions: Option<String>,
    openapi_version: Option<String>,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            lang: "rust".to_string(),
            out_dir: None,
            output_filename: None,
            client: true,
            server: true,
            mock: false,
            api_version: None,
            api_versions: None,
            openapi_version: None,
        }
    }
}

impl Builder {
    /// Creates a builder with the default configuration.
    ///
    /// Defaults:
    /// - language: `rust`
    /// - output directory: Cargo `OUT_DIR`
    /// - client generation: enabled
    /// - server generation: enabled
    /// - mock generation: disabled
    pub fn new() -> Self {
        Self::default()
    }

    /// Selects the target generator language.
    ///
    /// The exact accepted values are defined by `xidlc`, such as `rust`,
    /// `openapi`, and `openrpc`.
    pub fn with_lang(mut self, lang: impl Into<String>) -> Self {
        self.lang = lang.into();
        self
    }

    /// Overrides the output directory used by the generator.
    ///
    /// When not set, [`compile`](Self::compile) reads Cargo's `OUT_DIR`
    /// environment variable.
    pub fn with_out_dir(mut self, out_dir: impl Into<PathBuf>) -> Self {
        self.out_dir = Some(out_dir.into());
        self
    }

    /// Enables or disables mock generation.
    ///
    /// When enabled, generated traits will be annotated with `#[mockall::automock]`.
    /// In CLI mode this maps to the `--mock` flag, which only the client/server
    /// generator languages accept.
    pub fn with_mock(mut self, mock: bool) -> Self {
        self.mock = mock;
        self
    }

    /// Renames the generated single-file artifact after generation.
    ///
    /// This currently only works for generators that emit exactly one known file:
    /// `openapi` (`openapi_{filename}.json`) and `openrpc` (`openrpc.json`).
    ///
    /// Generation targets a private staging directory inside the output
    /// directory, so concurrent builds sharing one output directory cannot
    /// race on the default artifact name. The artifact is published at the
    /// requested path with an atomic rename once generation succeeds.
    ///
    /// Relative paths are resolved against the final output directory. Absolute
    /// paths are used as-is.
    pub fn with_output_filename(mut self, filename: impl Into<PathBuf>) -> Self {
        self.output_filename = Some(filename.into());
        self
    }

    /// Generates only this API version of a versioned contract.
    ///
    /// Accepts a `@version` enum member ident (`v2`) or its display value
    /// (`2.0`). Versioned contracts emit one module per version, so selecting
    /// one keeps SDK output single-versioned.
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = Some(version.into());
        self
    }

    /// Generates these API versions of a versioned contract.
    ///
    /// Accepts a comma separated list of member idents or display values, or
    /// `*` for every declared version.
    pub fn with_api_versions(mut self, versions: impl Into<String>) -> Self {
        self.api_versions = Some(versions.into());
        self
    }

    /// Selects the OpenAPI document version for the `openapi` generator.
    ///
    /// Accepts `auto` (the default, which upgrades the document to OpenAPI 3.2
    /// when the contract has streams) or `3.1` (which forces an OpenAPI 3.1
    /// document, rendering streams best-effort). Setting it for any other
    /// language is rejected.
    pub fn with_openapi_version(mut self, version: impl Into<String>) -> Self {
        self.openapi_version = Some(version.into());
        self
    }

    /// Enables or disables client-side artifact generation.
    pub fn with_client(mut self, value: bool) -> Self {
        self.client = value;
        self
    }

    /// Enables or disables server-side artifact generation.
    pub fn with_server(mut self, value: bool) -> Self {
        self.server = value;
        self
    }

    /// Runs the configured generator for the provided IDL input files.
    ///
    /// If no output directory was configured with
    /// [`with_out_dir`](Self::with_out_dir), this method requires Cargo's
    /// `OUT_DIR` environment variable to be present.
    ///
    /// Without the `bundle` feature, generation runs through the `xidlc`
    /// executable resolved from `PATH`; with `bundle`, the compiler runs
    /// in-process.
    ///
    /// When [`with_output_filename`](Self::with_output_filename) is set, the
    /// artifact is generated into a private staging directory and atomically
    /// published under the requested filename after generation succeeds.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `OUT_DIR` is required but missing
    /// - the backend configuration cannot be executed
    /// - the `xidlc` CLI is missing from `PATH` (default build)
    /// - code generation fails
    /// - the requested output rename is unsupported or cannot be completed
    pub fn compile(&self, inputs: &[impl AsRef<Path>]) -> Result<(), Error> {
        let out_dir = match &self.out_dir {
            Some(path) => path.clone(),
            None => PathBuf::from(
                env::var("OUT_DIR")
                    .map_err(|err| Error::Config(format!("OUT_DIR is not set: {err}")))?,
            ),
        };
        if self.output_filename.is_some() && out_dir == Path::new("-") {
            return Err(Error::Config(
                "with_output_filename is not supported when out_dir is '-'".to_string(),
            ));
        }
        let inputs: Vec<PathBuf> = inputs.iter().map(|p| p.as_ref().to_path_buf()).collect();
        // When an explicit output filename is requested, generate into a
        // process-private staging directory inside `out_dir` so the default
        // artifact name never becomes cross-process shared state (issue 458).
        // The rename into the final directory is then atomic, because the
        // staging directory shares `out_dir`'s filesystem.
        let staging_dir: Option<tempfile::TempDir> = if self.output_filename.is_some() {
            Self::default_artifact_name(&self.lang, &inputs)?;
            Some(
                tempfile::Builder::new()
                    .prefix(".xidl-build-stage-")
                    .tempdir_in(&out_dir)?,
            )
        } else {
            None
        };
        let generation_dir = staging_dir
            .as_ref()
            .map(|dir| dir.path().to_path_buf())
            .unwrap_or_else(|| out_dir.clone());
        let spec = GenerateSpec {
            lang: self.lang.clone(),
            out_dir: generation_dir.clone(),
            client: self.client,
            server: self.server,
            mock: self.mock,
            api_version: self.api_version.clone(),
            api_versions: self.api_versions.clone(),
            openapi_version: self.openapi_version.clone(),
            inputs,
        };
        #[cfg(feature = "bundle")]
        let result = BundleBackend.run(&spec);
        #[cfg(not(feature = "bundle"))]
        let result = CliBackend.run(&spec);
        result?;
        if let Some(custom_name) = &self.output_filename {
            self.apply_output_filename(&generation_dir, &out_dir, custom_name, &spec.inputs)?;
        }
        Ok(())
    }

    /// Versioned documents of a single-file generator, e.g.
    /// `openapi_petstore.v1.json` for `openapi_petstore.json`.
    fn versioned_documents(out_dir: &Path, default_name: &str) -> Result<Vec<PathBuf>, Error> {
        let Some(base) = default_name.strip_suffix(".json") else {
            return Ok(Vec::new());
        };
        let mut documents = Vec::new();
        for entry in fs::read_dir(out_dir)? {
            let path = entry?.path();
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let Some(rest) = file_name.strip_prefix(base) else {
                continue;
            };
            if rest.starts_with('.') && file_name.ends_with(".json") {
                documents.push(path);
            }
        }
        documents.sort();
        Ok(documents)
    }

    fn default_artifact_name(lang: &str, inputs: &[PathBuf]) -> Result<String, Error> {
        match lang {
            "openapi" => {
                let stem = inputs
                    .first()
                    .and_then(|p| p.file_stem())
                    .and_then(|s| s.to_str())
                    .unwrap_or("openapi");
                Ok(format!("openapi_{stem}.json"))
            }
            "openrpc" | "open-rpc" => {
                let stem = inputs
                    .first()
                    .and_then(|p| p.file_stem())
                    .and_then(|s| s.to_str())
                    .unwrap_or("openrpc");
                Ok(format!("openrpc_{stem}.json"))
            }
            _ => Err(Error::Config(format!(
                "with_output_filename is only supported for openapi/openrpc generators, got '{lang}'"
            ))),
        }
    }

    fn apply_output_filename(
        &self,
        generation_dir: &Path,
        out_dir: &Path,
        custom_name: &Path,
        inputs: &[PathBuf],
    ) -> Result<(), Error> {
        if out_dir == Path::new("-") {
            return Err(Error::Config(
                "with_output_filename is not supported when out_dir is '-'".to_string(),
            ));
        }

        let default_name = Self::default_artifact_name(self.lang.as_str(), inputs)?;

        let src = generation_dir.join(&default_name);
        let src = if src.exists() {
            src
        } else {
            let documents = Self::versioned_documents(generation_dir, &default_name)?;
            match documents.as_slice() {
                [] => {
                    return Err(Error::Compile(format!(
                        "generated file '{}' does not exist in '{}'",
                        default_name,
                        generation_dir.display()
                    )));
                }
                [single] => single.clone(),
                many => {
                    let names = many
                        .iter()
                        .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(Error::Compile(format!(
                        "generated output has one document per API version ({names}); \
                         select one with Builder::with_api_version"
                    )));
                }
            }
        };
        let dst = if custom_name.is_absolute() {
            custom_name.to_path_buf()
        } else {
            out_dir.join(custom_name)
        };
        if src == dst {
            return Ok(());
        }
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(src, dst)?;
        Ok(())
    }
}
