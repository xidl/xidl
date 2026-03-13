use crate::driver::lang::Plugin;
use crate::error::{IdlcError, IdlcResult};
use crate::jsonrpc::{Codegen, CodegenClient};
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
        let endpoint = Self::rpc_endpoint(lang)?;
        let (client, server) = match plugin {
            Plugin::Custom(custom_lang) => {
                let server = Self::spawn_custom_codegen_server(&custom_lang, endpoint.clone())?;
                let stream = Self::connect_with_retry(&endpoint).await?;
                let (reader, writer) = tokio::io::split(stream);
                let reader: RpcReader = Box::new(reader);
                let writer: RpcWriter = Box::new(writer);
                let client = CodegenClient::new(reader, writer);
                (client, server)
            }
            plugin => {
                let server = Self::spawn_builtin_codegen_server(plugin, endpoint.clone())?;
                let stream = Self::connect_with_retry(&endpoint).await?;
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
                Ok(tokio::spawn(async move {
                    let handler = crate::jsonrpc::CodegenServer::new($obj);
                    xidl_jsonrpc::Server::builder()
                        .with_service(handler)
                        .serve_on(&endpoint)
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
        endpoint: String,
    ) -> IdlcResult<JoinHandle<IdlcResult<()>>> {
        let exe = format!("xidl-{lang}");
        tracing::info!("{lang} is not a builtin supported language, try spawn {exe}");
        let mut child = std::process::Command::new(&exe)
            .arg("--endpoint")
            .arg(&endpoint)
            .spawn()
            .map_err(|err| std::io::Error::other(format!("cannot find plugin: {lang}, {err}")))?;

        let server = tokio::task::spawn_blocking(move || {
            child.wait()?;
            Ok(())
        });
        Ok(server)
    }

    async fn connect_with_retry(
        endpoint: &str,
    ) -> IdlcResult<Box<dyn xidl_jsonrpc::transport::Stream + Unpin + Send + 'static>> {
        let mut last_err = None;
        for _ in 0..50 {
            match xidl_jsonrpc::transport::connect(endpoint).await {
                Ok(stream) => return Ok(stream),
                Err(err) => {
                    last_err = Some(err);
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            }
        }
        let err = last_err.unwrap_or_else(|| {
            std::io::Error::other(format!("failed to connect rpc endpoint: {endpoint}"))
        });
        Err(IdlcError::rpc(err.to_string()))
    }

    fn rpc_endpoint(lang: &str) -> IdlcResult<String> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        #[cfg(unix)]
        {
            let path = std::path::Path::new("/tmp").join(format!("xidlc.{lang}.{nanos}.sock"));
            let path = path
                .into_os_string()
                .into_string()
                .map_err(|_| IdlcError::rpc("invalid ipc socket path".to_string()))?;
            Ok(format!("ipc://{path}"))
        }
        #[cfg(windows)]
        {
            let probe = std::net::TcpListener::bind("127.0.0.1:0")
                .map_err(|err| IdlcError::rpc(err.to_string()))?;
            let addr = probe
                .local_addr()
                .map_err(|err| IdlcError::rpc(err.to_string()))?;
            drop(probe);
            Ok(format!("tcp://{addr}"))
        }
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
