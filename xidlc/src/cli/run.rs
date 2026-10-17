use super::*;
use std::io::Write;

impl Cli {
    /// Executes the selected compiler command and preserves its exit contract.
    pub fn run(self) -> IdlcResult<()> {
        match self.command {
            Command::Lint(args) => args.execute(),
            Command::Gen(args) => {
                let help_command = args.lang.help_command();
                let args = args.into_driver_args()?;
                if args.files.is_empty() {
                    help_command.clone().print_help()?;
                    println!();
                    return Ok(());
                }
                switch_toolchain(&args.files)?;
                driver::Driver::run(args)
            }
            Command::Version(args) => match args.command {
                VersionCommand::Diff(args) => {
                    match crate::version_diff::VersionDiff::from_file(&args.idl)
                        .and_then(|diff| diff.compare(&args.from, &args.to))
                    {
                        Ok(Some(diff)) => {
                            let mut output = std::io::stdout().lock();
                            output
                                .write_all(diff.as_bytes())
                                .and_then(|_| output.flush())
                                .map_err(|error| IdlcError::CliExit {
                                    code: 2,
                                    message: Some(format!(
                                        "version diff: cannot write output for '{}': {error}",
                                        args.idl.display()
                                    )),
                                })?;
                            Err(IdlcError::CliExit {
                                code: 1,
                                message: None,
                            })
                        }
                        Ok(None) => Ok(()),
                        Err(error) => Err(IdlcError::CliExit {
                            code: 2,
                            message: Some(format!(
                                "version diff: contract '{}': {error}",
                                args.idl.display()
                            )),
                        }),
                    }
                }
            },
            Command::Fmt(args) => args.execute(),
            Command::Import(args) => {
                let out_dir = PathBuf::from(&args.out_dir);
                if !out_dir.exists() {
                    std::fs::create_dir_all(&out_dir)?;
                }
                match args.lang {
                    ImportLang::Openapi(files_args) => {
                        for file in files_args.files {
                            crate::import::openapi::import_openapi(&file, &out_dir)?;
                        }
                    }
                }
                Ok(())
            }
        }
    }
}
