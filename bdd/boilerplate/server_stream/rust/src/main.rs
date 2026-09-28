use async_trait::async_trait;
use xidl_jsonrpc::stream::{boxed, BoxStream};

mod gen { include!("../{{MODULE_NAME}}.rs"); }

struct MyFeed;

#[async_trait]
impl gen::Feed for MyFeed {
    async fn messages<'a>(
        &'a self,
        count: i32,
    ) -> Result<BoxStream<'a, gen::Message>, xidl_jsonrpc::Error> {
        Ok(boxed(async_stream::try_stream! {
            for seq in 0..count {
                yield gen::Message {
                    seq,
                    body: format!("body-{seq}"),
                };
            }
        }))
    }

    async fn ping<'a>(&'a self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("pong".to_string())
    }

    async fn echo(
        &self,
        mut stream: BoxStream<'static, gen::Message>,
    ) -> Result<BoxStream<'static, gen::Message>, xidl_jsonrpc::Error> {
        Ok(boxed(async_stream::try_stream! {
            while let Some(msg) = xidl_jsonrpc::futures_util::StreamExt::next(&mut stream).await {
                let msg = msg?;
                yield gen::Message {
                    seq: msg.seq + 100,
                    body: format!("echo:{}", msg.body),
                };
            }
        }))
    }

    async fn converse(
        &self,
        mut stream: BoxStream<'static, gen::Packet>,
    ) -> Result<BoxStream<'static, gen::Packet>, xidl_jsonrpc::Error> {
        Ok(boxed(async_stream::try_stream! {
            while let Some(msg) = xidl_jsonrpc::futures_util::StreamExt::next(&mut stream).await {
                let msg = msg?;
                match msg.tag() {
                    gen::PacketKind::PING => {
                        let id = *msg.as_ping_id();
                        yield gen::Packet::new_ping_id(id + 1);
                    }
                    gen::PacketKind::TEXT => {
                        let s = msg.as_data();
                        yield gen::Packet::new_data(format!("reply:{s}"));
                    }
                }
            }
        }))
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let server = xidl_jsonrpc::Server::builder()
        .with_service(gen::FeedServer::new(MyFeed))
        .with_endpoint(&format!("tcp://127.0.0.1:{}", port))
        .build()
        .await?;
    server.serve().await?;
    Ok(())
}
