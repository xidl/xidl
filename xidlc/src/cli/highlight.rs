use crate::error::{IdlcError, IdlcResult};
use clap::Args;
use std::path::{Path, PathBuf};

#[derive(Debug, Args)]
pub struct HighlightArgs {
    #[arg(long = "out-dir", short = 'o', default_value = "-")]
    out_dir: String,
    #[arg(long)]
    write: bool,
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
}

impl HighlightArgs {
    pub fn execute(self) -> IdlcResult<()> {
        if self.inputs.len() > 1 && self.out_dir == "-" && !self.write {
            return Err(IdlcError::fmt(
                "multiple inputs require --write or --out-dir".to_string(),
            ));
        }

        for (idx, input) in self.inputs.iter().enumerate() {
            let source = std::fs::read_to_string(input)?;
            let highlighted =
                crate::highlight::highlight_idl(&source, input.to_string_lossy().as_ref())?;
            if self.write {
                std::fs::write(input, highlighted.text)?;
            } else if self.out_dir == "-" {
                if idx > 0 {
                    println!();
                }
                print!("{}", highlighted.text);
            } else {
                let out_path = self.highlight_output_path(input);
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(out_path, highlighted.text)?;
            }
        }

        Ok(())
    }

    fn highlight_output_path(&self, input: &Path) -> PathBuf {
        let file_name = input
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("output.idl"));
        Path::new(&self.out_dir).join(file_name)
    }
}
