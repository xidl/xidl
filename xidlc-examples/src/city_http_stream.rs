include!(concat!(env!("OUT_DIR"), "/city_http_stream.rs"));

pub struct CityHttpStreamService;

#[async_trait::async_trait]
impl CityHttpStreamApi for CityHttpStreamService {
    async fn alerts(
        &self,
        req: xidl_rust_axum::Request<CityHttpStreamApiAlertsRequest>,
    ) -> Result<xidl_rust_axum::stream::SseStream<String>, xidl_rust_axum::Error> {
        let district = req.data.district;
        let lang = req.data.lang;
        let stream = xidl_rust_axum::futures_util::stream::iter(vec![
            Ok(format!("{district}:ALERT:1:{lang}")),
            Ok(format!("{district}:ALERT:2:{lang}")),
        ]);
        Ok(xidl_rust_axum::stream::boxed_sse(stream))
    }

    async fn ticker(
        &self,
        _req: xidl_rust_axum::Request<()>,
    ) -> Result<xidl_rust_axum::stream::SseStream<String>, xidl_rust_axum::Error> {
        let stream = xidl_rust_axum::futures_util::stream::iter(vec![
            Ok("tick-1".to_string()),
            Ok("tick-2".to_string()),
        ]);
        Ok(xidl_rust_axum::stream::boxed_sse(stream))
    }

    async fn maintenance_mode(
        &self,
        _req: xidl_rust_axum::Request<()>,
    ) -> Result<bool, xidl_rust_axum::Error> {
        Ok(false)
    }

    async fn set_maintenance_mode(
        &self,
        _req: xidl_rust_axum::Request<CityHttpStreamApiSetMaintenanceModeRequest>,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }

    async fn upload_asset(
        &self,
        req: xidl_rust_axum::Request<
            xidl_rust_axum::stream::NdjsonStream<CityHttpStreamApiUploadAssetRequest>,
        >,
    ) -> Result<String, xidl_rust_axum::Error> {
        let mut stream = req.data;
        let mut asset_id = String::new();
        let mut total = 0usize;
        while let Some(item) = xidl_rust_axum::futures_util::StreamExt::next(&mut stream).await {
            let item = item?;
            if asset_id.is_empty() {
                asset_id = item.asset_id;
            }
            total += item.chunk.len();
        }
        Ok(format!("uploaded:{asset_id}:{total}"))
    }

    async fn chat(
        &self,
        req: xidl_rust_axum::Request<
            xidl_rust_axum::stream::BidiServerStream<
                CityHttpStreamApiChatRequest,
                CityHttpStreamApiChatResponse,
            >,
        >,
    ) -> Result<(), xidl_rust_axum::Error> {
        let mut stream = req.data;
        while let Some(item) = stream.read().await {
            let item = item?;
            stream
                .write(CityHttpStreamApiChatResponse {
                    from: "server".to_string(),
                    text: format!("echo:{}:{}", item.room, item.message),
                })
                .await?;
        }
        stream.close();
        Ok(())
    }
}
