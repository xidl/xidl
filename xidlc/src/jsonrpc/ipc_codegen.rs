use super::Artifact;

#[async_trait::async_trait]
pub trait Codegen {
    async fn get_engine_version<'a>(&'a self) -> Result<String, xidl_jsonrpc::Error>;
    async fn get_properties<'a>(
        &'a self,
    ) -> Result<::xidl_parser::hir::ParserProperties, xidl_jsonrpc::Error>;
    async fn generate<'a>(
        &'a self,
        hir: ::xidl_parser::hir::Specification,
        path: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error>;
}

#[derive(::serde::Serialize, ::serde::Deserialize)]
struct CodegengetEngineVersionParams {}

#[derive(::serde::Serialize, ::serde::Deserialize)]
struct CodegengetPropertiesParams {}

#[derive(::serde::Serialize, ::serde::Deserialize)]
struct CodegengenerateParams {
    hir: ::xidl_parser::hir::Specification,
    path: String,
    props: ::xidl_parser::hir::ParserProperties,
}

#[derive(::serde::Serialize, ::serde::Deserialize)]
pub struct CodegengetEngineVersionResult {
    #[serde(rename = "return")]
    pub r#return: String,
}

#[derive(::serde::Serialize, ::serde::Deserialize)]
pub struct CodegengetPropertiesResult {
    #[serde(rename = "return")]
    pub r#return: ::xidl_parser::hir::ParserProperties,
}

#[derive(::serde::Serialize, ::serde::Deserialize)]
pub struct CodegengenerateResult {
    #[serde(rename = "return")]
    pub r#return: Vec<Artifact>,
}

pub struct CodegenServer<T> {
    inner: T,
}

impl<T> CodegenServer<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

#[async_trait::async_trait]
impl<T> xidl_jsonrpc::Handler for CodegenServer<T>
where
    T: Codegen + Send + Sync,
{
    async fn handle(
        &self,
        method: &str,
        params: ::serde_json::Value,
    ) -> Result<::serde_json::Value, xidl_jsonrpc::Error> {
        match method {
            "Codegen.get_engine_version" => {
                let params: CodegengetEngineVersionParams = ::serde_json::from_value(params)
                    .map_err(|err| xidl_jsonrpc::Error::invalid_params(err.to_string()))?;
                let result = self.inner.get_engine_version().await?;
                let wire = CodegengetEngineVersionResult { r#return: result };
                Ok(::serde_json::to_value(wire)?)
            }
            "Codegen.get_properties" => {
                let params: CodegengetPropertiesParams = ::serde_json::from_value(params)
                    .map_err(|err| xidl_jsonrpc::Error::invalid_params(err.to_string()))?;
                let result = self.inner.get_properties().await?;
                let wire = CodegengetPropertiesResult { r#return: result };
                Ok(::serde_json::to_value(wire)?)
            }
            "Codegen.generate" => {
                let params: CodegengenerateParams = ::serde_json::from_value(params)
                    .map_err(|err| xidl_jsonrpc::Error::invalid_params(err.to_string()))?;
                let result = self
                    .inner
                    .generate(params.hir, params.path, params.props)
                    .await?;
                let wire = CodegengenerateResult { r#return: result };
                Ok(::serde_json::to_value(wire)?)
            }
            _ => Err(xidl_jsonrpc::Error::method_not_found(method)),
        }
    }
}

pub struct CodegenClient<S> {
    client: tokio::sync::Mutex<xidl_jsonrpc::Client<S>>,
}

impl<S> CodegenClient<S>
where
    S: xidl_jsonrpc::transport::Stream + Unpin + Send,
{
    pub fn new(stream: S) -> Self {
        Self {
            client: tokio::sync::Mutex::new(xidl_jsonrpc::Client::new(stream)),
        }
    }
}

#[async_trait::async_trait]
impl<S> Codegen for CodegenClient<S>
where
    S: xidl_jsonrpc::transport::Stream + Unpin + Send,
{
    async fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        let params = CodegengetEngineVersionParams {};
        let mut client = self.client.lock().await;
        let result: CodegengetEngineVersionResult =
            client.call("Codegen.get_engine_version", params).await?;
        Ok(result.r#return)
    }

    async fn get_properties(
        &self,
    ) -> Result<::xidl_parser::hir::ParserProperties, xidl_jsonrpc::Error> {
        let params = CodegengetPropertiesParams {};
        let mut client = self.client.lock().await;
        let result: CodegengetPropertiesResult =
            client.call("Codegen.get_properties", params).await?;
        Ok(result.r#return)
    }

    async fn generate(
        &self,
        hir: ::xidl_parser::hir::Specification,
        path: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        let params = CodegengenerateParams { hir, path, props };
        let mut client = self.client.lock().await;
        let result: CodegengenerateResult = client.call("Codegen.generate", params).await?;
        Ok(result.r#return)
    }
}
