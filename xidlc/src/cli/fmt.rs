use crate::error::{IdlcError, IdlcResult};
use clap::Args;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum FormatLang {
    Idl,
    #[value(aliases = ["rs"])]
    Rust,
    C,
    #[value(aliases = ["cpp", "c++"])]
    Cpp,
}

#[derive(Debug, Args)]
pub struct FormatArgs {
    #[arg(long = "lang", short = 'l', value_enum, default_value_t = FormatLang::Idl)]
    lang: FormatLang,
    #[arg(long = "out-dir", short = 'o', default_value = "-")]
    out_dir: String,
    /// Format inplace.
    #[arg(long, short = 'i')]
    inplace: bool,
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
}

impl FormatArgs {
    pub fn execute(self) -> IdlcResult<()> {
        if self.inputs.len() > 1 && self.out_dir == "-" && !self.inplace {
            return Err(IdlcError::fmt(
                "multiple inputs require --write or --out-dir".to_string(),
            ));
        }

        for (idx, input) in self.inputs.iter().enumerate() {
            let source = std::fs::read_to_string(input)?;
            let formatted = match self.lang {
                FormatLang::Idl => crate::fmt::format_idl_source(&source)?,
                FormatLang::Rust => crate::fmt::format_rust_source(&source)?,
                FormatLang::C => crate::fmt::format_c_source(&source)?,
                FormatLang::Cpp => crate::fmt::format_c_source(&source)?,
            };
            if self.inplace {
                std::fs::write(input, formatted)?;
            } else if self.out_dir == "-" {
                if idx > 0 {
                    println!();
                }
                print!("{formatted}");
            } else {
                let out_path = self.format_output_path(input);
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(out_path, formatted)?;
            }
        }

        Ok(())
    }

    fn format_output_path(&self, input: &Path) -> PathBuf {
        let file_name = input
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("output.idl"));
        Path::new(&self.out_dir).join(file_name)
    }
}
