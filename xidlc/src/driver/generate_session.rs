use crate::driver::lang::Plugin;
use crate::error::IdlcResult;
use crate::jsonrpc::CodegenClient;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::task::JoinHandle;

mod support;

type RpcReader = Box<dyn AsyncRead + Unpin + Send>;
type RpcWriter = Box<dyn AsyncWrite + Unpin + Send>;
type RpcStream = Box<dyn xidl_jsonrpc::transport::Stream + Unpin + Send + 'static>;

struct SessionParts {
    client: CodegenClient<RpcReader, RpcWriter>,
    server: JoinHandle<IdlcResult<()>>,
}

pub struct CodegenSession {
    pub client: CodegenClient<RpcReader, RpcWriter>,
    server: JoinHandle<IdlcResult<()>>,
}

impl CodegenSession {
    pub async fn spawn(lang: &str) -> IdlcResult<Self> {
        let plugin = Plugin::from(lang);
        let session = match plugin {
            Plugin::Custom(custom_lang) => Self::spawn_custom_session(lang, &custom_lang).await?,
            plugin => Self::spawn_builtin_session(lang, plugin).await?,
        };
        support::verify_engine_version(&session.client).await?;
        Ok(Self {
            client: session.client,
            server: session.server,
        })
    }

    pub async fn finish(self) {
        drop(self.client);
        self.server.abort();
    }

    async fn spawn_custom_session(lang: &str, custom_lang: &str) -> IdlcResult<SessionParts> {
        let endpoint = support::rpc_endpoint(lang)?;
        let server = Self::spawn_custom_codegen_server(custom_lang, endpoint.clone())?;
        let stream = Self::connect_with_retry(&endpoint).await?;
        Ok(SessionParts {
            client: Self::client_from_stream(stream),
            server,
        })
    }

    async fn spawn_builtin_session(lang: &str, plugin: Plugin) -> IdlcResult<SessionParts> {
        let endpoint = support::random_inproc_endpoint(lang);
        let server = Self::spawn_builtin_codegen_server(plugin, endpoint.clone()).await?;
        let stream = Self::connect_inproc_with_retry(&endpoint).await?;
        Ok(SessionParts {
            client: Self::client_from_duplex_stream(stream),
            server,
        })
    }

    fn client_from_stream(stream: RpcStream) -> CodegenClient<RpcReader, RpcWriter> {
        let (reader, writer) = tokio::io::split(stream);
        CodegenClient::new(Box::new(reader), Box::new(writer))
    }

    fn client_from_duplex_stream(
        stream: tokio::io::DuplexStream,
    ) -> CodegenClient<RpcReader, RpcWriter> {
        let (reader, writer) = tokio::io::split(stream);
        CodegenClient::new(Box::new(reader), Box::new(writer))
    }

    async fn spawn_builtin_codegen_server(
        lang: Plugin,
        endpoint: String,
    ) -> IdlcResult<JoinHandle<IdlcResult<()>>> {
        macro_rules! run_server {
            ($obj:expr) => {{
                let handler = crate::jsonrpc::CodegenServer::new($obj);
                let server = xidl_jsonrpc::Server::builder()
                    .with_service(handler)
                    .with_endpoint(format!("inproc://{endpoint}"))
                    .build()
                    .await
                    .map_err(|err| crate::error::IdlcError::rpc(err.to_string()))?;
                Ok(tokio::spawn(async move {
                    server
                        .serve()
                        .await
                        .map_err(|err| crate::error::IdlcError::rpc(err.to_string()))
                }))
            }};
        }

        #[allow(unreachable_patterns)]
        match lang {
            Plugin::Hir => run_server!(crate::generate::hir_gen::HirGen),
            Plugin::HttpHir => run_server!(crate::generate::http_hir::HttpHirCodegen),
            Plugin::TypedAst => run_server!(crate::generate::typed_ast_gen::TypedAstGen),
            #[cfg(feature = "gen-c")]
            Plugin::C => run_server!(crate::generate::c::CCodegen),
            #[cfg(feature = "gen-cpp")]
            Plugin::Cpp => run_server!(crate::generate::cpp::CppCodegen),
            #[cfg(feature = "gen-go")]
            Plugin::Go => run_server!(crate::generate::go::GoCodegen),
            #[cfg(feature = "gen-go-http")]
            Plugin::GoHttp => run_server!(crate::generate::go_http::GoHttpCodegen),
            #[cfg(feature = "gen-python")]
            Plugin::Python => run_server!(crate::generate::python::PythonCodegen),
            #[cfg(feature = "gen-python-http")]
            Plugin::PythonHttp => run_server!(crate::generate::python_http::PythonHttpCodegen),
            #[cfg(feature = "gen-rust")]
            Plugin::Rust => run_server!(crate::generate::rust::RustCodegen),
            #[cfg(feature = "gen-rust-jsonrpc")]
            Plugin::RustJsonRpc => run_server!(crate::generate::rust_jsonrpc::RustJsonRpcCodegen),
            #[cfg(feature = "gen-rust-axum")]
            Plugin::Axum => run_server!(crate::generate::rust_axum::RustAxumCodegen),
            #[cfg(feature = "gen-openapi")]
            Plugin::Openapi => run_server!(crate::generate::openapi::OpenApiCodegen),
            #[cfg(feature = "gen-openrpc")]
            Plugin::Openrpc => run_server!(crate::generate::openrpc::OpenRpcCodegen),
            #[cfg(feature = "gen-typescript")]
            Plugin::Typescript => run_server!(crate::generate::typescript::TypescriptCodegen),
            Plugin::Custom(_) => unreachable!("custom plugins use spawn_custom_codegen_server"),
            var => panic!("does not support {var:?}"),
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

    async fn connect_with_retry(endpoint: &str) -> IdlcResult<RpcStream> {
        support::retry_connect(
            || xidl_jsonrpc::transport::connect(endpoint),
            format!("failed to connect rpc endpoint: {endpoint}"),
        )
        .await
    }

    async fn connect_inproc_with_retry(endpoint: &str) -> IdlcResult<tokio::io::DuplexStream> {
        support::retry_connect(
            || std::future::ready(xidl_jsonrpc::transport::connect_inproc(endpoint)),
            "failed to connect inproc endpoint".to_string(),
        )
        .await
    }
}
