#[cfg(test)]
mod tests;

use crate::cli::CliArgs;
use crate::error::{IdlcError, IdlcResult};
use crate::jsonrpc::{Codegen, CodegenClient};
use crate::unnamed_pipe::{Reader, Writer};
use semver::{Version, VersionReq};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::BufReader;
use tokio::task::JoinHandle;

pub struct File {
    path: String,
    content: String,
}

impl File {
    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    pub fn content(&self) -> &str {
        self.content.as_str()
    }
}

pub struct Driver {
    args: CliArgs,
}

impl Driver {
    pub async fn run(args: CliArgs) -> IdlcResult<()> {
        Self { args }.execute().await
    }

    async fn execute(self) -> IdlcResult<()> {
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
            let files = generator
                .generate_from_idl(&source, &input, props.clone())
                .await?;
            output.write_files(files)?;
        }

        Ok(())
    }
}

#[cfg(test)]
pub async fn generate_from_idl(
    source: &str,
    path: &Path,
    lang: &str,
    props: HashMap<String, serde_json::Value>,
) -> IdlcResult<Vec<File>> {
    let mut generator = Generator::new(lang);
    generator.generate_from_idl(source, path, props).await
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

    pub async fn generate_from_idl(
        &mut self,
        source: &str,
        path: &Path,
        mut props: HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
        tracing::info!("generate for idl");
        props.insert("idl".into(), source.into());
        props.insert("target_lang".into(), self.lang.clone().into());
        props.insert("xidlc_version".into(), env!("CARGO_PKG_VERSION").into());
        let ts = if cfg!(test) || cfg!(target_os = "emscripten") {
            0
        } else {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        };
        props.insert("xidlc_timestamp".into(), ts.into());
        let target_props = self.get_properties_for_lang().await?;
        merge_properties(&mut props, target_props);
        let empty = xidl_parser::hir::Specification(vec![]);
        self.generate_for_lang("hir", empty, path, props).await
    }

    async fn generate_for_lang(
        &mut self,
        lang: &str,
        hir: xidl_parser::hir::Specification,
        input: &Path,
        mut props: HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
        tracing::info!("generate for lang: {lang}");
        if !props.contains_key("xidlc_version") {
            props.insert("xidlc_version".into(), env!("CARGO_PKG_VERSION").into());
        }
        if !props.contains_key("xidlc_timestamp") {
            let ts = if cfg!(test) || cfg!(target_os = "emscripten") {
                0
            } else {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            };
            props.insert("xidlc_timestamp".into(), ts.into());
        }
        let input_str = input.to_string_lossy();
        let session = CodegenSession::spawn(lang).await?;
        let properties: HashMap<String, serde_json::Value> = session
            .client
            .get_properties()
            .await
            .map_err(|err| IdlcError::rpc(err.to_string()))?;
        merge_properties(&mut props, properties);

        let artifacts: Vec<crate::jsonrpc::Artifact> = session
            .client
            .generate(hir, input_str.to_string(), props)
            .await
            .map_err(|err| IdlcError::rpc(err.to_string()))?;

        let mut ret: Vec<File> = vec![];
        for file in artifacts {
            match file.tag() {
                crate::jsonrpc::ArtifactKind::Hir => {
                    let data = file.into_hir();
                    ret.extend(
                        Box::pin(self.generate_for_lang(
                            &data.lang,
                            data.hir,
                            input,
                            data.props.clone(),
                        ))
                        .await?,
                    );
                }
                crate::jsonrpc::ArtifactKind::File => {
                    let data = file.into_file();
                    ret.push(File {
                        path: data.path.clone(),
                        content: data.content.clone(),
                    })
                }
            }
        }
        session.finish().await;
        Ok(ret)
    }

    async fn get_properties_for_lang(&mut self) -> IdlcResult<HashMap<String, serde_json::Value>> {
        tracing::info!("get properties for {}", self.lang);
        if !self.properties.is_empty() {
            return Ok(self.properties.clone());
        }
        let session = CodegenSession::spawn(&self.lang).await?;
        let props: HashMap<String, serde_json::Value> = session
            .client
            .get_properties()
            .await
            .map_err(|err| IdlcError::rpc(err.to_string()))
            .unwrap();
        session.finish().await;
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
    client: CodegenClient<BufReader<Reader>, Writer>,
    server: JoinHandle<IdlcResult<()>>,
}

impl CodegenSession {
    async fn spawn(lang: &str) -> IdlcResult<Self> {
        #[cfg(target_os = "emscripten")]
        {
            let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
            let (client_read, client_write) = tokio::io::split(client_stream);
            let (server_read, server_write) = tokio::io::split(server_stream);
            let server = Self::spawn_inprocess_server(lang, server_read, server_write)?;
            let client = CodegenClient::new(BufReader::new(client_read), client_write);
            Self::verify_engine_version(&client).await?;
            return Ok(Self { client, server });
        }

        #[cfg(not(target_os = "emscripten"))]
        {
            let (stdout_tx, stdout_rx) = crate::unnamed_pipe::pipe()?;
            let (stdin_tx, stdin_rx) = crate::unnamed_pipe::pipe()?;
            let server = Self::spawn_codegen_server(lang, stdout_rx, stdin_tx)?;
            let reader = BufReader::new(stdin_rx);
            let writer = stdout_tx;
            let client = CodegenClient::new(reader, writer);
            Self::verify_engine_version(&client).await?;
            Ok(Self { client, server })
        }
    }

    async fn finish(self) {
        drop(self.client);
        match self.server.await {
            Ok(Err(err)) => {
                eprintln!("codegen server failed: {}", err);
            }
            Ok(Ok(())) => {}
            Err(err) => {
                eprintln!("codegen server task failed: {}", err);
            }
        }
    }

    #[cfg(not(target_os = "emscripten"))]
    fn spawn_codegen_server(
        lang: &str,
        stdout_rx: crate::unnamed_pipe::Reader,
        stdin_tx: crate::unnamed_pipe::Writer,
    ) -> IdlcResult<JoinHandle<IdlcResult<()>>> {
        macro_rules! run_server {
            ($obj:expr) => {
                Ok(tokio::spawn(async move {
                    let io = xidl_jsonrpc::Io::new(BufReader::new(stdout_rx), stdin_tx);
                    let handler = crate::jsonrpc::CodegenServer::new($obj);
                    xidl_jsonrpc::Server::builder()
                        .with_listener(xidl_jsonrpc::MuxListener::from_io(io))
                        .with_service(handler)
                        .serve()
                        .await
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
            "ts" | "typescript" => run_server!(crate::generate::typescript::TypescriptCodegen),
            #[cfg(target_os = "emscripten")]
            _ => {
                unreachable!()
            }
            #[cfg(not(target_os = "emscripten"))]
            _ => {
                let exe = format!("xidl-{lang}");
                let mut child = std::process::Command::new(&exe)
                    .stdin(stdin_tx.into_owned_fd())
                    .stdout(stdout_rx.into_owned_fd())
                    .spawn()?;

                let server = tokio::task::spawn_blocking(move || {
                    child.wait()?;
                    Ok(())
                });
                Ok(server)
            }
        }
    }

    #[cfg(target_os = "emscripten")]
    fn spawn_inprocess_server<R, W>(
        lang: &str,
        reader: R,
        writer: W,
    ) -> IdlcResult<JoinHandle<IdlcResult<()>>>
    where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
        W: tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        macro_rules! run_server {
            ($obj:expr) => {
                Ok(tokio::spawn(async move {
                    let io = xidl_jsonrpc::Io::new(BufReader::new(reader), writer);
                    let handler = crate::jsonrpc::CodegenServer::new($obj);
                    xidl_jsonrpc::Server::builder()
                        .with_listener(xidl_jsonrpc::MuxListener::from_io(io))
                        .with_service(handler)
                        .serve()
                        .await
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
            "ts" | "typescript" => run_server!(crate::generate::typescript::TypescriptCodegen),
            _ => unreachable!(),
        }
    }

    async fn verify_engine_version(
        client: &CodegenClient<BufReader<Reader>, Writer>,
    ) -> IdlcResult<()> {
        let engine_req: String = client
            .get_engine_version()
            .await
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
