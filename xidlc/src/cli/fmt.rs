use crate::error::IdlcResult;
use clap::Args;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum FormatLang {
    Idl,
    #[value(aliases = ["rs"])]
    Rust,
    C,
    #[value(aliases = ["cpp", "c++"])]
    Cpp,
    #[value(aliases = ["ts"])]
    TypeScript,
}

#[derive(Debug, Args)]
pub struct ArgsFormat {
    #[arg(long = "lang", short = 'l', value_enum, default_value_t = FormatLang::Idl)]
    lang: FormatLang,
    /// Format inplace.
    #[arg(long, short = 'i')]
    inplace: bool,
    #[arg(required = true)]
    files: Vec<PathBuf>,
}

impl ArgsFormat {
    pub fn execute(self) -> IdlcResult<()> {
        for file in self.files.iter() {
            let source = std::fs::read_to_string(file)?;
            let formatted = match self.lang {
                FormatLang::Idl => crate::fmt::format_idl_source(&source)?,
                FormatLang::Rust => crate::fmt::format_rust_source(&source)?,
                FormatLang::C | FormatLang::Cpp => crate::fmt::format_c_source(&source)?,
                FormatLang::TypeScript => crate::fmt::format_typescript_source(&source)?,
            };

            if self.inplace {
                std::fs::write(file, formatted)?;
            } else {
                println!("{}", formatted);
            }
        }

        Ok(())
    }
}
