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
    /// Relative paths are resolved against the final output directory. Absolute
    /// paths are used as-is.
    pub fn with_output_filename(mut self, filename: impl Into<PathBuf>) -> Self {
        self.output_filename = Some(filename.into());
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
    /// generated artifact is renamed after successful code generation.
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
        let spec = GenerateSpec {
            lang: self.lang.clone(),
            out_dir,
            client: self.client,
            server: self.server,
            mock: self.mock,
            inputs: inputs.iter().map(|p| p.as_ref().to_path_buf()).collect(),
        };
        #[cfg(feature = "bundle")]
        let result = BundleBackend.run(&spec);
        #[cfg(not(feature = "bundle"))]
        let result = CliBackend.run(&spec);
        result?;
        if let Some(custom_name) = &self.output_filename {
            self.apply_output_filename(&spec.out_dir, custom_name, &spec.inputs)?;
        }
        Ok(())
    }

    fn apply_output_filename(
        &self,
        out_dir: &Path,
        custom_name: &Path,
        inputs: &[PathBuf],
    ) -> Result<(), Error> {
        if out_dir == Path::new("-") {
            return Err(Error::Config(
                "with_output_filename is not supported when out_dir is '-'".to_string(),
            ));
        }

        let default_name = match self.lang.as_str() {
            "openapi" => {
                let stem = inputs
                    .first()
                    .and_then(|p| p.file_stem())
                    .and_then(|s| s.to_str())
                    .unwrap_or("openapi");
                format!("openapi_{stem}.json")
            }
            "openrpc" | "open-rpc" => "openrpc.json".to_string(),
            _ => {
                return Err(Error::Config(format!(
                    "with_output_filename is only supported for openapi/openrpc generators, got '{}'",
                    self.lang
                )));
            }
        };

        let src = out_dir.join(&default_name);
        if !src.exists() {
            return Err(Error::Compile(format!(
                "generated file '{}' does not exist in '{}'",
                default_name,
                out_dir.display()
            )));
        }

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
