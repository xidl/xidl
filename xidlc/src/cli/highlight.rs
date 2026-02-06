use crate::error::IdlcResult;
use clap::Args;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct ArgsHighlight {
    files: Vec<PathBuf>,
}

impl ArgsHighlight {
    pub fn execute(self) -> IdlcResult<()> {
        for (idx, input) in self.files.iter().enumerate() {
            let source = std::fs::read_to_string(input)?;
            let highlighted =
                crate::highlight::highlight_idl(&source, input.to_string_lossy().as_ref())?;

            if idx > 0 {
                println!();
            }
            print!("{}", highlighted.text);
        }

        Ok(())
    }
}
