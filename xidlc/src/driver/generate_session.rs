use crate::error::{IdlcError, IdlcResult};
use crate::jsonrpc::{Codegen, CodegenClient};
use crate::transport::{Reader, Writer};
use semver::{Version, VersionReq};
use tokio::io::BufReader;
use tokio::task::JoinHandle;

pub struct CodegenSession {
    pub client: CodegenClient<BufReader<Reader>, Writer>,
    server: JoinHandle<IdlcResult<()>>,
}

impl CodegenSession {
    pub async fn spawn(lang: &str) -> IdlcResult<Self> {
        let (stdout_tx, stdout_rx) = crate::transport::pipe()?;
        let (stdin_tx, stdin_rx) = crate::transport::pipe()?;
        let server = Self::spawn_codegen_server(lang, stdout_rx, stdin_tx)?;
        let reader = BufReader::new(stdin_rx);
        let writer = stdout_tx;
        let client = CodegenClient::new(reader, writer);
        Self::verify_engine_version(&client).await?;
        Ok(Self { client, server })
    }

    pub async fn finish(self) {
        drop(self.client);
        self.server.abort();
    }

    fn spawn_codegen_server(
        lang: &str,
        stdout_rx: crate::transport::Reader,
        stdin_tx: crate::transport::Writer,
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
            "c" | "cc" => run_server!(crate::generate::c::CCodegen),
            "cpp" | "cxx" | "c++" => run_server!(crate::generate::cpp::CppCodegen),
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
                tracing::debug!("spawn {exe}");
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
