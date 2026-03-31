//! Build-script support for invoking `xidlc` from `build.rs`.
//!
//! This crate wraps the `xidlc` code generator in a small builder API so Cargo
//! build scripts can generate code or schema artifacts without shelling out to
//! the `xidlc` binary.
//!
//! # Example
//!
//! ```no_run
//! fn main() -> Result<(), xidl_build::IdlcError> {
//!     println!("cargo:rerun-if-changed=idl/hello_world.idl");
//!
//!     xidl_build::Builder::new()
//!         .with_lang("rust")
//!         .compile(&["idl/hello_world.idl"])?;
//!
//!     Ok(())
//! }
//! ```
//!
//! By default, generated files are written to Cargo's `OUT_DIR`, and both
//! client and server artifacts are enabled.
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// The error type returned by the underlying `xidlc` compiler driver.
pub use xidlc::error::IdlcError;

/// Configures and runs IDL code generation from a Cargo build script.
///
/// `Builder` provides a small fluent interface around `xidlc`'s generator
/// options. The default configuration targets Rust output, writes into
/// `OUT_DIR`, and enables both client and server generation.
///
/// # Example
///
/// ```no_run
/// fn main() -> Result<(), xidl_build::IdlcError> {
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
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            lang: "rust".to_string(),
            out_dir: None,
            output_filename: None,
            client: true,
            server: true,
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

    /// Renames the generated single-file artifact after generation.
    ///
    /// This currently only works for generators that emit exactly one known file:
    /// `openapi` (`openapi.json`) and `openrpc` (`openrpc.json`).
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
}

impl Builder {
    /// Runs the configured generator for the provided IDL input files.
    ///
    /// If no output directory was configured with
    /// [`with_out_dir`](Self::with_out_dir), this method requires Cargo's
    /// `OUT_DIR` environment variable to be present.
    ///
    /// When [`with_output_filename`](Self::with_output_filename) is set, the
    /// generated artifact is renamed after successful code generation.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `OUT_DIR` is required but missing
    /// - code generation fails
    /// - the requested output rename is unsupported or cannot be completed
    pub fn compile(&self, inputs: &[impl AsRef<Path>]) -> Result<(), IdlcError> {
        let out_dir = match &self.out_dir {
            Some(path) => path.clone(),
            None => PathBuf::from(
                env::var("OUT_DIR")
                    .map_err(|err| IdlcError::fmt(format!("OUT_DIR is not set: {err}")))?,
            ),
        };
        let args = xidlc::cli::ArgsGenerate {
            lang: self.lang.clone(),
            out_dir: out_dir.to_string_lossy().to_string(),
            files: inputs.iter().map(|p| p.as_ref().to_path_buf()).collect(),
            client: self.client,
            server: self.server,
            dry_run: false,
        };

        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { xidlc::driver::Driver::run(args).await })?;

        if let Some(custom_name) = &self.output_filename {
            self.apply_output_filename(&out_dir, custom_name)?;
        }

        Ok(())
    }

    fn apply_output_filename(&self, out_dir: &Path, custom_name: &Path) -> Result<(), IdlcError> {
        if out_dir == Path::new("-") {
            return Err(IdlcError::fmt(
                "with_output_filename is not supported when out_dir is '-'",
            ));
        }
        let Some(default_name) = default_single_artifact_name(&self.lang) else {
            return Err(IdlcError::fmt(format!(
                "with_output_filename is only supported for openapi/openrpc generators, got '{}'",
                self.lang
            )));
        };
        let src = out_dir.join(default_name);
        if !src.exists() {
            return Err(IdlcError::fmt(format!(
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

fn default_single_artifact_name(lang: &str) -> Option<&'static str> {
    let lang = lang.trim().to_ascii_lowercase();
    match lang.as_str() {
        "openapi" => Some("openapi.json"),
        "openrpc" | "open-rpc" => Some("openrpc.json"),
        _ => None,
    }
}
