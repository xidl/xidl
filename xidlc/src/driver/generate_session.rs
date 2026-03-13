use crate::driver::lang::Plugin;
use crate::error::{IdlcError, IdlcResult};
use crate::jsonrpc::{Codegen, CodegenClient};
use crate::transport::{Reader, Writer};
use semver::{Version, VersionReq};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::task::JoinHandle;

type RpcReader = Box<dyn AsyncRead + Unpin + Send>;
type RpcWriter = Box<dyn AsyncWrite + Unpin + Send>;

pub struct CodegenSession {
    pub client: CodegenClient<RpcReader, RpcWriter>,
    server: JoinHandle<IdlcResult<()>>,
}

impl CodegenSession {
    pub async fn spawn(lang: &str) -> IdlcResult<Self> {
        let plugin = Plugin::from(lang);
        let (client, server) = match plugin {
            Plugin::Custom(custom_lang) => {
                let (stdout_tx, stdout_rx) = crate::transport::pipe()?;
                let (stdin_tx, stdin_rx) = crate::transport::pipe()?;
                let server = Self::spawn_custom_codegen_server(&custom_lang, stdout_rx, stdin_tx)?;
                let reader: RpcReader = Box::new(stdin_rx);
                let writer: RpcWriter = Box::new(stdout_tx);
                let client = CodegenClient::new(reader, writer);
                (client, server)
            }
            plugin => {
                let endpoint = Self::random_inproc_endpoint(lang);
                let server = Self::spawn_builtin_codegen_server(plugin, endpoint.clone())?;
                let stream = Self::connect_inproc_with_retry(&endpoint).await?;
                let (reader, writer) = tokio::io::split(stream);
                let reader: RpcReader = Box::new(reader);
                let writer: RpcWriter = Box::new(writer);
                let client = CodegenClient::new(reader, writer);
                (client, server)
            }
        };
        Self::verify_engine_version(&client).await?;
        Ok(Self { client, server })
    }

    pub async fn finish(self) {
        drop(self.client);
        self.server.abort();
    }

    fn spawn_builtin_codegen_server(
        lang: Plugin,
        endpoint: String,
    ) -> IdlcResult<JoinHandle<IdlcResult<()>>> {
        macro_rules! run_server {
            ($obj:expr) => {{
                let listener = xidl_jsonrpc::transport::InprocListener::bind(endpoint.clone())
                    .map_err(|err| crate::error::IdlcError::rpc(err.to_string()))?;
                Ok(tokio::spawn(async move {
                    let handler = crate::jsonrpc::CodegenServer::new($obj);
                    xidl_jsonrpc::Server::builder()
                        .with_listener(listener)
                        .with_service(handler)
                        .serve()
                        .await
                        .map_err(|err| crate::error::IdlcError::rpc(err.to_string()))
                }))
            }};
        }

        match lang {
            Plugin::Hir => run_server!(crate::generate::hir_gen::HirGen),
            Plugin::TypedAst => run_server!(crate::generate::typed_ast_gen::TypedAstGen),
            Plugin::C => run_server!(crate::generate::c::CCodegen),
            Plugin::Cpp => run_server!(crate::generate::cpp::CppCodegen),
            Plugin::Rust => run_server!(crate::generate::rust::RustCodegen),
            Plugin::RustJsonRpc => {
                run_server!(crate::generate::rust_jsonrpc::RustJsonRpcCodegen)
            }
            Plugin::Axum => {
                run_server!(crate::generate::rust_axum::RustAxumCodegen)
            }
            Plugin::Openapi => {
                run_server!(crate::generate::openapi::OpenApiCodegen)
            }
            Plugin::Openrpc => {
                run_server!(crate::generate::openrpc::OpenRpcCodegen)
            }
            Plugin::Typescript => run_server!(crate::generate::typescript::TypescriptCodegen),
            Plugin::Custom(_) => unreachable!("custom plugins use spawn_custom_codegen_server"),
        }
    }

    fn spawn_custom_codegen_server(
        lang: &str,
        stdout_rx: Reader,
        stdin_tx: Writer,
    ) -> IdlcResult<JoinHandle<IdlcResult<()>>> {
        let exe = format!("xidl-{lang}");
        eprintln!("{exe}");
        tracing::info!("{lang} is not a builtin supported language,try spawn {exe}");
        let mut child = std::process::Command::new(&exe)
            .stdin(stdin_tx.into_stdio())
            .stdout(stdout_rx.into_stdio())
            .spawn()
            .map_err(|err| std::io::Error::other(format!("cannot find plugin: {lang}, {err}")))?;

        let server = tokio::task::spawn_blocking(move || {
            child.wait()?;
            Ok(())
        });
        Ok(server)
    }

    async fn connect_inproc_with_retry(endpoint: &str) -> IdlcResult<tokio::io::DuplexStream> {
        let mut last_err = None;
        for _ in 0..50 {
            match xidl_jsonrpc::transport::connect_inproc(endpoint) {
                Ok(stream) => return Ok(stream),
                Err(err) => {
                    last_err = Some(err);
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            }
        }
        let err =
            last_err.unwrap_or_else(|| std::io::Error::other("failed to connect inproc endpoint"));
        Err(IdlcError::rpc(err.to_string()))
    }

    fn random_inproc_endpoint(lang: &str) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        format!("xidlc.codegen.{lang}.{nanos}")
    }

    async fn verify_engine_version(client: &CodegenClient<RpcReader, RpcWriter>) -> IdlcResult<()> {
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
