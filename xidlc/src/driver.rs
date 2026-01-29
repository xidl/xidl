#[cfg(test)]
mod tests;

use crate::cli::CliArgs;
use crate::error::{IdlcError, IdlcResult};
use crate::jsonrpc::{Codegen, CodegenClient};
use std::collections::HashMap;
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::process::Command;
use std::thread::{self, JoinHandle};

pub struct File {
    path: String,
    content: String,
}

pub fn run(args: CliArgs) -> IdlcResult<()> {
    if args.out_dir != "-" {
        fs::create_dir_all(&args.out_dir)?;
    }

    for input in args.inputs {
        let source = fs::read_to_string(&input)?;
        let files = generate_from_idl(&source, &input, &args.lang)?;
        write_files(&args.out_dir, files)?;
    }

    Ok(())
}

fn generate_from_idl(source: &str, path: &Path, lang: &str) -> IdlcResult<Vec<File>> {
    let mut props: HashMap<String, serde_json::Value> = HashMap::new();
    props.insert("idl".into(), source.into());
    props.insert("target_lang".into(), lang.into());

    let empty = xidl_parser::hir::Specification(vec![]);

    let target_props = get_properties_for_lang(lang)?;
    props.extend(target_props);

    generate_for_lang("hir", empty, path, props)
}

fn generate_for_lang(
    lang: &str,
    hir: xidl_parser::hir::Specification,
    input: &Path,
    props: HashMap<String, serde_json::Value>,
) -> IdlcResult<Vec<File>> {
    let input_str = input.to_string_lossy();

    let (stdout_tx, stdout_rx) = interprocess::unnamed_pipe::pipe()?;
    let (stdin_tx, stdin_rx) = interprocess::unnamed_pipe::pipe()?;

    let server = spawn_codegen_server(lang, stdout_rx, stdin_tx)?;

    let reader = BufReader::new(stdin_rx);
    let writer = stdout_tx;

    let client = CodegenClient::new(reader, writer);
    let mut properties = client
        .get_properties()
        .map_err(|err| IdlcError::rpc(err.to_string()))?;

    properties.extend(props);

    let artifacts = client
        .generate(hir, input_str.to_string(), properties)
        .map_err(|err| IdlcError::rpc(err.to_string()))?;

    let mut ret: Vec<File> = vec![];

    drop(client);
    if let Err(err) = server.join().unwrap() {
        eprintln!("codegen server failed: {}", err);
    }

    for file in artifacts {
        match file.tag() {
            crate::jsonrpc::ArtifactKind::Hir => {
                let data = unsafe { file.into_hir() };
                ret.extend(generate_for_lang(
                    &data.lang,
                    data.hir,
                    input,
                    data.props.clone(),
                )?);
            }
            crate::jsonrpc::ArtifactKind::File => {
                let data = unsafe { file.into_file() };
                ret.push(File {
                    path: data.path.clone(),
                    content: data.content.clone(),
                })
            }
        }
    }

    Ok(ret)
}

fn get_properties_for_lang(lang: &str) -> IdlcResult<HashMap<String, serde_json::Value>> {
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
    let client = CodegenClient::new(reader, writer);
    client
        .get_properties()
        .map_err(|err| IdlcError::rpc(err.to_string()))
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
        "hir" => run_server!(crate::generate::hir_gen::HirGen),
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

fn write_files(out_dir: &str, files: Vec<File>) -> IdlcResult<()> {
    let mut order = Vec::new();
    let mut merged: HashMap<String, String> = HashMap::new();
    for file in files {
        let File { path, content } = file;
        if let Some(existing) = merged.get_mut(&path) {
            existing.push_str(&content);
        } else {
            order.push(path.clone());
            merged.insert(path, content);
        }
    }

    let out_dir_path = Path::new(out_dir);

    for path in order {
        let content = merged.remove(&path).unwrap_or_default();
        let file_path = Path::new(&path);
        let out_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            out_dir_path.join(file_path)
        };
        if out_dir != "-" {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
        }
        tracing::info!("write file: {}", out_path.display());
        if out_dir == "-" {
            println!("{}", content);
        } else {
            fs::write(out_path, content)?;
        }
    }
    Ok(())
}
