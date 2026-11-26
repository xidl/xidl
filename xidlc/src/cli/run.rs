use super::*;

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
            Command::Version(args) => args.execute(),
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
