use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub use xidlc::error::IdlcError;

#[derive(Clone, Debug)]
pub struct Builder {
    lang: String,
    out_dir: Option<PathBuf>,
    output_filename: Option<PathBuf>,
    client: bool,
    server: bool,
    ts: bool,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            lang: "rust".to_string(),
            out_dir: None,
            output_filename: None,
            client: true,
            server: true,
            ts: true,
        }
    }
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_lang(mut self, lang: impl Into<String>) -> Self {
        self.lang = lang.into();
        self
    }

    pub fn with_out_dir(mut self, out_dir: impl Into<PathBuf>) -> Self {
        self.out_dir = Some(out_dir.into());
        self
    }

    pub fn with_output_filename(mut self, filename: impl Into<PathBuf>) -> Self {
        self.output_filename = Some(filename.into());
        self
    }

    pub fn with_client(mut self, value: bool) -> Self {
        self.client = value;
        self
    }

    pub fn with_server(mut self, value: bool) -> Self {
        self.server = value;
        self
    }
}

impl Builder {
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
            ts: self.ts,
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

    fn apply_output_filename(
        &self,
        out_dir: &Path,
        custom_name: &Path,
    ) -> Result<(), IdlcError> {
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
