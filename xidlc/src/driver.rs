use crate::cli::CliArgs;
use crate::error::Result;
use crate::generate::{self, GeneratedFile};
use crate::ipc;
use std::fs;
use std::path::Path;

pub fn run(args: CliArgs) -> Result<()> {
    fs::create_dir_all(&args.out_dir)?;

    for input in args.inputs {
        let source = fs::read_to_string(&input)?;
        let typed = xidl_parser::parser::parser_text(&source)?;
        let hir = xidl_parser::hir::Specification::from(typed);
        let files = generate_for_lang(&args.lang, &hir, &input)?;
        write_files(&args.out_dir, files)?;
    }

    Ok(())
}

fn generate_for_lang(
    lang: &str,
    hir: &xidl_parser::hir::Specification,
    input: &Path,
) -> Result<Vec<GeneratedFile>> {
    match lang {
        "c" => generate::c::generate(hir, input),
        other => ipc::generate(other, hir),
    }
}

fn write_files(out_dir: &Path, files: Vec<GeneratedFile>) -> Result<()> {
    for file in files {
        let path = out_dir.join(file.filename);
        fs::write(path, file.filecontent)?;
    }
    Ok(())
}
