use crate::gen;
use async_trait::async_trait;
use xidl_rust_axum::Error;

pub(crate) struct Store;
macro_rules! service {
    ($version:ident) => {
        #[async_trait]
        impl gen::$version::business::Store for Store {
            async fn get(
                &self,
                id: i32,
                search: String,
                theme: String,
            ) -> xidl_rust_axum::Result<gen::$version::business::Item> {
                assert_eq!(search, "a +/%");
                assert_eq!(theme, "dark");
                Ok(gen::$version::business::Item { id: id.to_string() })
            }
            async fn remove(&self, _id: i32) -> xidl_rust_axum::Result<()> {
                Ok(())
            }
            async fn fail(
                &self,
                id: i32,
            ) -> Result<gen::$version::business::Item, gen::$version::business::StoreFailError>
            {
                if id == 1 {
                    Err(gen::$version::business::StoreFailError::Conflict(
                        gen::$version::business::Conflict {
                            message: "conflict".into(),
                        },
                    ))
                } else {
                    Err(Error::new(500, "storage offline").into())
                }
            }
            async fn chat(
                &self,
                req: xidl_rust_axum::Request<
                    xidl_rust_axum::stream::BidiServerStream<
                        gen::$version::business::StoreChatRequest,
                        gen::$version::business::StoreChatResponse,
                    >,
                >,
                room: String,
            ) -> xidl_rust_axum::Result<()> {
                assert_eq!(room, "team +/%");
                let mut stream = req.data;
                while let Some(value) = stream.read().await {
                    stream
                        .write(gen::$version::business::StoreChatResponse {
                            reply: format!("{}:ack", value?.text),
                        })
                        .await?;
                }
                Ok(())
            }
        }
        #[async_trait]
        impl gen::$version::business::Health for Store {
            async fn health(&self) -> xidl_rust_axum::Result<String> {
                Ok("healthy".into())
            }
        }
        #[async_trait]
        impl gen::$version::business::DuplexShapes for Store {
            async fn scalar(
                &self,
                req: xidl_rust_axum::Request<
                    xidl_rust_axum::stream::BidiServerStream<
                        gen::$version::business::DuplexShapesScalarRequest,
                        String,
                    >,
                >,
            ) -> xidl_rust_axum::Result<()> {
                let mut stream = req.data;
                if let Some(value) = stream.read().await {
                    let value = value?;
                    assert_eq!(value.text, "hello");
                    stream.write("hello:scalar".into()).await?;
                }
                Ok(())
            }
            async fn model(
                &self,
                req: xidl_rust_axum::Request<
                    xidl_rust_axum::stream::BidiServerStream<
                        gen::$version::business::DuplexShapesModelRequest,
                        gen::$version::business::Item,
                    >,
                >,
            ) -> xidl_rust_axum::Result<()> {
                let mut stream = req.data;
                if let Some(value) = stream.read().await {
                    let value = value?;
                    assert_eq!(value.id, "object");
                    stream
                        .write(gen::$version::business::Item { id: value.id })
                        .await?;
                }
                Ok(())
            }
            async fn sink(
                &self,
                req: xidl_rust_axum::Request<
                    xidl_rust_axum::stream::BidiServerStream<
                        gen::$version::business::DuplexShapesSinkRequest,
                        (),
                    >,
                >,
            ) -> xidl_rust_axum::Result<()> {
                let mut stream = req.data;
                if let Some(value) = stream.read().await {
                    assert_eq!(value?.text, "stored");
                }
                Ok(())
            }
            async fn source(
                &self,
                req: xidl_rust_axum::Request<xidl_rust_axum::stream::BidiServerStream<(), String>>,
            ) -> xidl_rust_axum::Result<()> {
                let mut stream = req.data;
                stream.write("announcement".into()).await?;
                Ok(())
            }
        }
    };
}
service!(v1);
service!(v2);
