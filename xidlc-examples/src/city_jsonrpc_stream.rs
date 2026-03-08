include!(concat!(env!("OUT_DIR"), "/city_jsonrpc_stream.rs"));

pub struct CityJsonrpcStreamService;

#[async_trait::async_trait]
impl CityJsonrpcStreamApi for CityJsonrpcStreamService {
    async fn alerts<'a>(
        &'a self,
        district: String,
    ) -> Result<xidl_jsonrpc::stream::BoxStream<'a, serde_json::Value>, xidl_jsonrpc::Error> {
        let stream = async_stream::try_stream! {
            yield serde_json::json!({ "message": format!("{district}:alert-1") });
            yield serde_json::json!({ "message": format!("{district}:alert-2") });
        };
        Ok(xidl_jsonrpc::stream::boxed(stream))
    }

    async fn upload_sensor<'a>(
        &'a self,
        mut stream: xidl_jsonrpc::stream::BoxStream<'a, serde_json::Value>,
    ) -> Result<(), xidl_jsonrpc::Error> {
        while let Some(item) = xidl_rust_axum::futures_util::StreamExt::next(&mut stream).await {
            let _ = item?;
        }
        Ok(())
    }

    async fn chat(
        &self,
    ) -> Result<
        xidl_jsonrpc::stream::DuplexStream<serde_json::Value, serde_json::Value>,
        xidl_jsonrpc::Error,
    > {
        Err(xidl_jsonrpc::Error::Protocol(
            "server-side bidi runtime is not implemented",
        ))
    }

    async fn get_attribute_ops_notice(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("ok".to_string())
    }

    async fn set_attribute_ops_notice<'a>(
        &'a self,
    ) -> Result<xidl_jsonrpc::stream::BoxStream<'a, String>, xidl_jsonrpc::Error> {
        let stream = async_stream::try_stream! {
            yield "notice-1".to_string();
            yield "notice-2".to_string();
        };
        Ok(xidl_jsonrpc::stream::boxed(stream))
    }
}
