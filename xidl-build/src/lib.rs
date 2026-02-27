use std::env;
use std::path::{Path, PathBuf};

pub use xidlc::error::IdlcError;

pub trait XidlBuild {
    fn compile(&self, inputs: &[impl AsRef<Path>]) -> Result<(), IdlcError>;
}

#[derive(Clone, Debug)]
pub struct Builder {
    lang: String,
    out_dir: Option<PathBuf>,
    client: bool,
    server: bool,
    ts: bool,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            lang: "rust".to_string(),
            out_dir: None,
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

    pub fn lang(mut self, lang: impl Into<String>) -> Self {
        self.lang = lang.into();
        self
    }

    pub fn out_dir(mut self, out_dir: impl Into<PathBuf>) -> Self {
        self.out_dir = Some(out_dir.into());
        self
    }

    pub fn skip_client(mut self, value: bool) -> Self {
        self.client = value;
        self
    }

    pub fn skip_server(mut self, value: bool) -> Self {
        self.server = value;
        self
    }
}

impl XidlBuild for Builder {
    fn compile(&self, inputs: &[impl AsRef<Path>]) -> Result<(), IdlcError> {
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
        };

        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { xidlc::driver::Driver::run(args).await })
    }
}
