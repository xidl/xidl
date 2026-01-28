#[cfg(test)]
mod tests;

use crate::cli::CliArgs;
use crate::error::{IdlcError, IdlcResult};
use crate::generate::GeneratedFile;
use crate::jsonrpc::{Codegen, CodegenClient};
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::process::Command;
use std::thread::{self, JoinHandle};

pub fn run(args: CliArgs) -> IdlcResult<()> {
    fs::create_dir_all(&args.out_dir)?;

    for input in args.inputs {
        let source = fs::read_to_string(&input)?;
        let files = generate_for_lang(&args.lang, &source, &input)?;
        write_files(&args.out_dir, files)?;
    }

    Ok(())
}

fn generate_for_lang(lang: &str, source: &str, input: &Path) -> IdlcResult<Vec<GeneratedFile>> {
    let input_str = input.to_string_lossy();

    let (stdout_tx, stdout_rx) = interprocess::unnamed_pipe::pipe()?;
    let (stdin_tx, stdin_rx) = interprocess::unnamed_pipe::pipe()?;

    let server = spawn_codegen_server(lang, stdout_rx, stdin_tx)?;

    let reader = BufReader::new(stdin_rx);
    let writer = stdout_tx;
    let files = {
        let client = CodegenClient::new(reader, writer);
        let properties = client
            .parser_properties()
            .map_err(|err| IdlcError::rpc(err.to_string()))?;
        let typed = xidl_parser::parser::parser_text(source)?;
        let hir =
            xidl_parser::hir::Specification::from_typed_ast_with_properties(typed, properties);
        client
            .generate(hir, input_str.to_string())
            .map_err(|err| IdlcError::rpc(err.to_string()))?
    };

    let server_result = server
        .join()
        .map_err(|_| IdlcError::rpc("c server thread panicked"))?;

    server_result?;
    Ok(files)
}

pub fn spawn_codegen_server(
    lang: &str,
    stdout_rx: interprocess::unnamed_pipe::Recver,
    stdin_tx: interprocess::unnamed_pipe::Sender,
) -> IdlcResult<JoinHandle<IdlcResult<()>>> {
    match lang {
        "c" => {
            let server = thread::spawn(move || {
                let reader = BufReader::new(stdout_rx);
                crate::generate::c::serve_jsonrpc(reader, stdin_tx)
            });

            Ok(server)
        }
        "cpp" => {
            let server = thread::spawn(move || {
                let reader = BufReader::new(stdout_rx);
                crate::generate::cpp::serve_jsonrpc(reader, stdin_tx)
            });

            Ok(server)
        }
        "rust" | "rs" => {
            let server = thread::spawn(move || {
                let reader = BufReader::new(stdout_rx);
                crate::generate::rust::serve_jsonrpc(reader, stdin_tx)
            });

            Ok(server)
        }
        "rust_jsonrpc" => {
            let server = thread::spawn(move || {
                let reader = BufReader::new(stdout_rx);
                crate::generate::rust_jsonrpc::serve_jsonrpc(reader, stdin_tx)
            });

            Ok(server)
        }
        _ => {
            let exe = format!("xidl-{lang}");
            let mut child = Command::new(&exe)
                .stdin(std::os::fd::OwnedFd::from(stdin_tx))
                .stdout(std::os::fd::OwnedFd::from(stdout_rx))
                .spawn()?;

            let server = std::thread::spawn(move || {
                child.wait()?;
                Ok(())
            });
            Ok(server)
        }
    }
}

fn write_files(out_dir: &Path, files: Vec<GeneratedFile>) -> IdlcResult<()> {
    for file in files {
        let path = out_dir.join(file.filename);
        fs::write(path, file.filecontent)?;
    }
    Ok(())
}
