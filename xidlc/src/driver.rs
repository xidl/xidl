#[cfg(test)]
mod tests;

use crate::cli::CliArgs;
use crate::error::{IdlcError, IdlcResult};
use crate::generate::Artifact;
use crate::jsonrpc::{Codegen, CodegenClient};
use std::collections::HashMap;
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

fn generate_for_lang(lang: &str, source: &str, input: &Path) -> IdlcResult<Vec<Artifact>> {
    let input_str = input.to_string_lossy();

    let (stdout_tx, stdout_rx) = interprocess::unnamed_pipe::pipe()?;
    let (stdin_tx, stdin_rx) = interprocess::unnamed_pipe::pipe()?;

    let server = spawn_codegen_server(lang, stdout_rx, stdin_tx)?;
    scopeguard::defer! {
        if let Err(err) = server.join().unwrap() {
            eprintln!("codegen server failed: {}", err);
        }
    }

    let reader = BufReader::new(stdin_rx);
    let writer = stdout_tx;
    let (artifacts, global_properties) = {
        let client = CodegenClient::new(reader, writer);
        let properties = client
            .get_properties()
            .map_err(|err| IdlcError::rpc(err.to_string()))?;
        let global_properties = properties.clone();
        let typed = xidl_parser::parser::parser_text(source)?;
        let hir =
            xidl_parser::hir::Specification::from_typed_ast_with_properties(typed, properties);
        let artifacts = client
            .generate(hir, input_str.to_string())
            .map_err(|err| IdlcError::rpc(err.to_string()))?;
        (artifacts, global_properties)
    };

    resolve_artifacts_with_properties(artifacts, input, global_properties)
}

pub fn spawn_codegen_server(
    lang: &str,
    stdout_rx: interprocess::unnamed_pipe::Recver,
    stdin_tx: interprocess::unnamed_pipe::Sender,
) -> IdlcResult<JoinHandle<IdlcResult<()>>> {
    macro_rules! run_server {
        ($obj:expr) => {
            Ok(thread::spawn(move || {
                let io = xidl_jsonrpc::Io::new(BufReader::new(stdout_rx), stdin_tx);
                let handler = crate::jsonrpc::CodegenServer::new($obj);
                xidl_jsonrpc::Server::builder()
                    .with_io(io)
                    .with_service(handler)
                    .serve()
                    .map_err(|err| crate::error::IdlcError::rpc(err.to_string()))
            }))
        };
    }

    match lang {
        "c" => run_server!(crate::generate::c::CCodegen),
        "cpp" => run_server!(crate::generate::cpp::CppCodegen),
        "rust" | "rs" => run_server!(crate::generate::rust::RustCodegen),
        "rs_jsonrpc" | "rust_jsonrpc" => {
            run_server!(crate::generate::rust_jsonrpc::RustJsonRpcCodegen)
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

fn generate_from_hir(
    lang: &str,
    hir: xidl_parser::hir::Specification,
    input: &Path,
    _properties: &xidl_parser::hir::ParserProperties,
) -> IdlcResult<Vec<Artifact>> {
    let input_str = input.to_string_lossy();
    let (stdout_tx, stdout_rx) = interprocess::unnamed_pipe::pipe()?;
    let (stdin_tx, stdin_rx) = interprocess::unnamed_pipe::pipe()?;

    let server = spawn_codegen_server(lang, stdout_rx, stdin_tx)?;
    let reader = BufReader::new(stdin_rx);
    let writer = stdout_tx;

    let artifacts = {
        let client = CodegenClient::new(reader, writer);
        client
            .generate(hir, input_str.to_string())
            .map_err(|err| IdlcError::rpc(err.to_string()))?
    };

    let server_result = server
        .join()
        .map_err(|_| IdlcError::rpc("c server thread panicked"))?;
    server_result?;
    Ok(artifacts)
}

fn merge_properties(
    base: &xidl_parser::hir::ParserProperties,
    injected: &xidl_parser::hir::ParserProperties,
) -> xidl_parser::hir::ParserProperties {
    let mut merged = base.clone();
    for (key, value) in injected {
        merged.insert(key.clone(), value.clone());
    }
    merged
}

fn resolve_artifacts_with_properties(
    artifacts: Vec<Artifact>,
    input: &Path,
    global_properties: xidl_parser::hir::ParserProperties,
) -> IdlcResult<Vec<Artifact>> {
    let mut out = Vec::new();
    for artifact in artifacts {
        match artifact {
            Artifact::File { path, content } => out.push(Artifact::File { path, content }),
            Artifact::Hir {
                lang,
                hir,
                properties,
            } => {
                let merged = merge_properties(&global_properties, &properties);
                let nested = if lang == "rs" || lang == "rust" {
                    crate::generate::rust::generate_with_properties(&hir, input, &merged)?
                } else {
                    generate_from_hir(&lang, hir, input, &merged)?
                };
                let nested = resolve_artifacts_with_properties(nested, input, merged)?;
                out.extend(nested);
            }
        }
    }
    Ok(out)
}

fn write_files(out_dir: &Path, files: Vec<Artifact>) -> IdlcResult<()> {
    let mut order = Vec::new();
    let mut merged: HashMap<String, String> = HashMap::new();
    for file in files {
        let (path, content) = match file {
            Artifact::File { path, content } => (path, content),
            Artifact::Hir { .. } => {
                return Err(IdlcError::rpc(
                    "unresolved Hir artifact reached write_files",
                ));
            }
        };
        if let Some(existing) = merged.get_mut(&path) {
            existing.push_str(&content);
        } else {
            order.push(path.clone());
            merged.insert(path, content);
        }
    }

    for path in order {
        let content = merged.remove(&path).unwrap_or_default();
        let file_path = Path::new(&path);
        let out_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            out_dir.join(file_path)
        };
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(out_path, content)?;
    }
    Ok(())
}
