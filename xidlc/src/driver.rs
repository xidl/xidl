#[cfg(test)]
mod tests;

use crate::cli::CliArgs;
use crate::error::{IdlcError, IdlcResult};
use crate::jsonrpc::{Codegen, CodegenClient};
use semver::{Version, VersionReq};
use std::collections::HashMap;
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::process::Command;
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct File {
    path: String,
    content: String,
}

pub struct Driver {
    args: CliArgs,
}

impl Driver {
    pub fn run(args: CliArgs) -> IdlcResult<()> {
        Self { args }.execute()
    }

    fn execute(self) -> IdlcResult<()> {
        let output = OutputTarget::new(&self.args.out_dir)?;
        let mut generator = Generator::new(&self.args.lang);
        let mut props: HashMap<String, serde_json::Value> = HashMap::new();
        if self.args.skip_client {
            props.insert("enable_client".into(), false.into());
        }
        if self.args.skip_server {
            props.insert("enable_server".into(), false.into());
        }

        for input in self.args.inputs {
            let source = fs::read_to_string(&input)?;
            let files = generator.generate_from_idl(&source, &input, props.clone())?;
            output.write_files(files)?;
        }

        Ok(())
    }
}

#[cfg(test)]
pub fn generate_from_idl(
    source: &str,
    path: &Path,
    lang: &str,
    props: HashMap<String, serde_json::Value>,
) -> IdlcResult<Vec<File>> {
    let mut generator = Generator::new(lang);
    generator.generate_from_idl(source, path, props)
}

pub struct Generator {
    lang: String,
    properties: HashMap<String, serde_json::Value>,
}

impl Generator {
    pub fn new(lang: &str) -> Self {
        Self {
            lang: lang.to_string(),
            properties: HashMap::new(),
        }
    }

    pub fn generate_from_idl(
        &mut self,
        source: &str,
        path: &Path,
        mut props: HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
        props.insert("idl".into(), source.into());
        props.insert("target_lang".into(), self.lang.clone().into());
        props.insert("xidlc_version".into(), env!("CARGO_PKG_VERSION").into());
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        props.insert("xidlc_timestamp".into(), ts.into());
        let target_props = self.get_properties_for_lang()?;
        merge_properties(&mut props, target_props);
        let empty = xidl_parser::hir::Specification(vec![]);
        self.generate_for_lang("hir", empty, path, props)
    }

    fn generate_for_lang(
        &mut self,
        lang: &str,
        hir: xidl_parser::hir::Specification,
        input: &Path,
        mut props: HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
        let input_str = input.to_string_lossy();
        let session = CodegenSession::spawn(lang)?;
        let properties = session
            .client
            .get_properties()
            .map_err(|err| IdlcError::rpc(err.to_string()))?;
        merge_properties(&mut props, properties);

        let artifacts = session
            .client
            .generate(hir, input_str.to_string(), props)
            .map_err(|err| IdlcError::rpc(err.to_string()))?;

        let mut ret: Vec<File> = vec![];
        for file in artifacts {
            match file.tag() {
                crate::jsonrpc::ArtifactKind::Hir => {
                    let data = unsafe { file.into_hir() };
                    ret.extend(self.generate_for_lang(
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
        session.finish();
        Ok(ret)
    }

    fn get_properties_for_lang(&mut self) -> IdlcResult<HashMap<String, serde_json::Value>> {
        if !self.properties.is_empty() {
            return Ok(self.properties.clone());
        }
        let session = CodegenSession::spawn(&self.lang)?;
        let props = session
            .client
            .get_properties()
            .map_err(|err| IdlcError::rpc(err.to_string()))?;
        session.finish();
        self.properties = props.clone();
        Ok(props)
    }
}

fn merge_properties(
    props: &mut HashMap<String, serde_json::Value>,
    incoming: HashMap<String, serde_json::Value>,
) {
    for (k, v) in incoming {
        props.entry(k).or_insert(v);
    }
}

struct CodegenSession {
    client: CodegenClient<
        BufReader<interprocess::unnamed_pipe::Recver>,
        interprocess::unnamed_pipe::Sender,
    >,
    server: JoinHandle<IdlcResult<()>>,
}

impl CodegenSession {
    fn spawn(lang: &str) -> IdlcResult<Self> {
        let (stdout_tx, stdout_rx) = interprocess::unnamed_pipe::pipe()?;
        let (stdin_tx, stdin_rx) = interprocess::unnamed_pipe::pipe()?;
        let server = Self::spawn_codegen_server(lang, stdout_rx, stdin_tx)?;
        let reader = BufReader::new(stdin_rx);
        let writer = stdout_tx;
        let client = CodegenClient::new(reader, writer);
        Self::verify_engine_version(&client)?;
        Ok(Self { client, server })
    }

    fn finish(self) {
        drop(self.client);
        if let Err(err) = self.server.join().unwrap() {
            eprintln!("codegen server failed: {}", err);
        }
    }

    fn spawn_codegen_server(
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
            "typed_ast" | "typed-ast" => run_server!(crate::generate::typed_ast_gen::TypedAstGen),
            "c" => run_server!(crate::generate::c::CCodegen),
            "cpp" => run_server!(crate::generate::cpp::CppCodegen),
            "rust" | "rs" => run_server!(crate::generate::rust::RustCodegen),
            "rs_jsonrpc" | "rust_jsonrpc" | "rs-jsonrpc" | "rust-jsonrpc" => {
                run_server!(crate::generate::rust_jsonrpc::RustJsonRpcCodegen)
            }
            "rs_axum" | "rust_axum" | "rs-axum" | "rust-axum" => {
                run_server!(crate::generate::rust_axum::RustAxumCodegen)
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

    fn verify_engine_version(
        client: &CodegenClient<
            BufReader<interprocess::unnamed_pipe::Recver>,
            interprocess::unnamed_pipe::Sender,
        >,
    ) -> IdlcResult<()> {
        let engine_req = client
            .get_engine_version()
            .map_err(|err| IdlcError::rpc(err.to_string()))?;
        let req = VersionReq::parse(&engine_req).map_err(|err| {
            IdlcError::rpc(format!(
                "invalid engine version requirement \"{engine_req}\": {err}"
            ))
        })?;
        let version = Version::parse(env!("CARGO_PKG_VERSION")).map_err(|err| {
            IdlcError::rpc(format!(
                "invalid xidlc version \"{}\": {err}",
                env!("CARGO_PKG_VERSION")
            ))
        })?;
        if !req.matches(&version) {
            return Err(IdlcError::rpc(format!(
                "xidlc {version} is not compatible with engine requirement {engine_req}"
            )));
        }
        Ok(())
    }
}

struct OutputTarget {
    out_dir: String,
}

impl OutputTarget {
    fn new(out_dir: &str) -> IdlcResult<Self> {
        if out_dir != "-" {
            fs::create_dir_all(out_dir)?;
        }
        Ok(Self {
            out_dir: out_dir.to_string(),
        })
    }

    fn write_files(&self, files: Vec<File>) -> IdlcResult<()> {
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

        let out_dir_path = Path::new(&self.out_dir);

        for path in order {
            let content = merged.remove(&path).unwrap_or_default();
            let file_path = Path::new(&path);
            let out_path = if file_path.is_absolute() {
                file_path.to_path_buf()
            } else {
                out_dir_path.join(file_path)
            };
            if self.out_dir != "-"
                && let Some(parent) = out_path.parent()
            {
                fs::create_dir_all(parent)?;
            }
            tracing::info!("write file: {}", out_path.display());
            if self.out_dir == "-" {
                println!("{}", content);
            } else {
                fs::write(out_path, content)?;
            }
        }
        Ok(())
    }
}
