use xidl_jsonrpc::futures_util::StreamExt;

include!(concat!(env!("OUT_DIR"), "/city_jsonrpc_stream.rs"));

pub struct CityJsonrpcStreamService;

#[async_trait::async_trait]
impl CityJsonrpcStreamApi for CityJsonrpcStreamService {
    async fn alerts<'a>(
        &'a self,
        district: String,
    ) -> Result<xidl_jsonrpc::stream::BoxStream<'a, String>, xidl_jsonrpc::Error> {
        Ok(async_stream::try_stream! {
            for i in 0..2 {
                yield format!("{district}:alert-{i}");
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
        mut stream: xidl_jsonrpc::stream::BoxStream<'static, CityJsonrpcStreamApichatParams>,
    ) -> Result<
        xidl_jsonrpc::stream::BoxStream<'static, CityJsonrpcStreamApichatResult>,
        xidl_jsonrpc::Error,
    > {
        Ok(xidl_jsonrpc::stream::boxed(async_stream::try_stream! {
            while let Some(item) = stream.next().await {
                let item = item?;
                yield CityJsonrpcStreamApichatResult {
                    from: "server".to_string(),
                    text: format!("echo:{}:{}", item.room_id, item.text),
                };
            }
        }))
    }

    async fn packet_stream(
        &self,
        mut stream: xidl_jsonrpc::stream::BoxStream<'static, Packet>,
    ) -> Result<xidl_jsonrpc::stream::BoxStream<'static, Packet>, xidl_jsonrpc::Error> {
        Ok(xidl_jsonrpc::stream::boxed(async_stream::try_stream! {
            while let Some(item) = stream.next().await {
                let item = item?;
                match item.tag() {
                    PacketKind::HEARTBEAT => {
                        let seq = *item.as_seq();
                        yield Packet::new_seq(seq + 1);
                    }
                    PacketKind::DATA => {
                        let payload = item.as_payload();
                        yield Packet::new_payload(format!("echo:{payload}"));
                    }
                }
            }
        }))
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
