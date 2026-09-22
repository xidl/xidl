use crate::driver::lang::Plugin;
use crate::error::IdlcResult;
use crate::jsonrpc::{Artifact, Codegen, CodegenInput, GenerateParams, PluginClient, RpcError};
use crate::macros::log_info;
use xidl_parser::hir::ParserProperties;

mod support;

enum Inner {
    Builtin(Box<dyn Codegen + Send + Sync>),
    External(PluginClient),
}

/// One generator stage. Built-ins are called in-process; external plugins are
/// spoken to over the NDJSON stdio protocol.
pub struct CodegenSession {
    inner: Inner,
}

impl CodegenSession {
    pub fn spawn(lang: &str) -> IdlcResult<Self> {
        let plugin = Plugin::from(lang);
        let mut session = match plugin {
            Plugin::Custom(custom_lang) => Self::spawn_custom_session(&custom_lang)?,
            plugin => Self::spawn_builtin_session(plugin)?,
        };
        support::verify_engine_version(&mut session)?;
        Ok(session)
    }

    pub fn get_engine_version(&mut self) -> Result<String, RpcError> {
        match &mut self.inner {
            Inner::Builtin(generator) => generator.get_engine_version(),
            Inner::External(client) => client.call("get_engine_version", serde_json::Value::Null),
        }
    }

    pub fn get_properties(&mut self) -> Result<ParserProperties, RpcError> {
        match &mut self.inner {
            Inner::Builtin(generator) => generator.get_properties(),
            Inner::External(client) => client.call("get_properties", serde_json::Value::Null),
        }
    }

    pub fn generate(
        &mut self,
        input: CodegenInput,
        path: String,
        props: ParserProperties,
    ) -> Result<Vec<Artifact>, RpcError> {
        match &mut self.inner {
            Inner::Builtin(generator) => generator.generate(input, path, props),
            Inner::External(client) => {
                let params = GenerateParams { input, path, props };
                client.call("generate", serde_json::to_value(params)?)
            }
        }
    }

    pub fn finish(self) {
        match self.inner {
            Inner::Builtin(_) => {}
            Inner::External(mut client) => client.shutdown(),
        }
    }

    fn spawn_custom_session(lang: &str) -> IdlcResult<Self> {
        let exe = format!("xidl-{lang}");
        log_info!("{lang} is not a builtin supported language, try spawn {exe}");
        let client = PluginClient::spawn(&exe)
            .map_err(|err| std::io::Error::other(format!("cannot find plugin: {lang}, {err}")))?;
        Ok(Self {
            inner: Inner::External(client),
        })
    }

    fn spawn_builtin_session(plugin: Plugin) -> IdlcResult<Self> {
        #[allow(unreachable_patterns)]
        let generator: Box<dyn Codegen + Send + Sync> = match plugin {
            Plugin::Hir => Box::new(crate::generate::hir_gen::HirGen),
            Plugin::RestHir => Box::new(crate::generate::rest_hir_gen::RestHirCodegen),
            Plugin::TypedAst => Box::new(crate::generate::typed_ast_gen::TypedAstGen),
            #[cfg(feature = "gen-go")]
            Plugin::Go => Box::new(crate::generate::go::GoCodegen),
            #[cfg(feature = "gen-go-rest")]
            Plugin::GoRest => Box::new(crate::generate::go_rest::GoRestCodegen),
            #[cfg(feature = "gen-rust")]
            Plugin::Rust => Box::new(crate::generate::rust::RustCodegen),
            #[cfg(feature = "gen-rust-jsonrpc")]
            Plugin::RustJsonRpc => Box::new(crate::generate::rust_jsonrpc::RustJsonRpcCodegen),
            #[cfg(feature = "gen-rust-axum")]
            Plugin::Axum => Box::new(crate::generate::rust_axum::RustAxumCodegen),
            #[cfg(feature = "gen-openapi")]
            Plugin::Openapi => Box::new(crate::generate::openapi::OpenApiCodegen),
            #[cfg(feature = "gen-openrpc")]
            Plugin::Openrpc => Box::new(crate::generate::openrpc::OpenRpcCodegen),
            #[cfg(feature = "gen-typescript")]
            Plugin::Typescript => Box::new(crate::generate::typescript::TypescriptCodegen),
            #[cfg(feature = "gen-typescript-rest")]
            Plugin::TypescriptRest => {
                Box::new(crate::generate::typescript_rest::TypescriptRestCodegen)
            }
            Plugin::Custom(_) => unreachable!("custom plugins use spawn_custom_session"),
            var => panic!("does not support {var:?}"),
        };
        Ok(Self {
            inner: Inner::Builtin(generator),
        })
    }
}
