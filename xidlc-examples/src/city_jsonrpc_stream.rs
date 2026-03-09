use xidl_jsonrpc::futures_util::StreamExt;

include!(concat!(env!("OUT_DIR"), "/city_jsonrpc_stream.rs"));

pub struct CityJsonrpcStreamService;

#[async_trait::async_trait]
impl CityJsonrpcStreamApi for CityJsonrpcStreamService {
    async fn alerts<'a>(
        &'a self,
        district: String,
    ) -> Result<xidl_jsonrpc::stream::BoxStream<'a, serde_json::Value>, xidl_jsonrpc::Error> {
        Ok(async_stream::try_stream! {
            for i in 0..2{
                yield serde_json::json!({ "message": format!("{district}:alert-{i}") });
            }
        }
        .boxed())
    }

    async fn upload_sensor<'a>(
        &'a self,
        mut stream: xidl_jsonrpc::stream::BoxStream<'a, serde_json::Value>,
    ) -> Result<(), xidl_jsonrpc::Error> {
        while let Some(item) = stream.next().await {
            let _ = item?;
        }
        Ok(())
    }

    async fn chat(
        &self,
        mut stream: xidl_jsonrpc::stream::ReaderWriter<serde_json::Value, serde_json::Value>,
    ) -> Result<(), xidl_jsonrpc::Error> {
        while let Some(item) = stream.read().await {
            let value = item?;
            let room = value
                .get("room_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let text = value
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            stream
                .write(serde_json::json!({
                    "from": "server",
                    "text": format!("echo:{room}:{text}")
                }))
                .await?;
        }
        Ok(())
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
