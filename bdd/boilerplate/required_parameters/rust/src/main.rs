use async_trait::async_trait;
use xidl_rust_axum::Result;

pub mod gen {
    include!("../{{MODULE_NAME}}.rs");
}

struct Parameters;

impl Parameters {
    fn inputs(count: i32, active: bool, label: String) -> gen::Inputs {
        gen::Inputs {
            count,
            active,
            label,
            note: None,
        }
    }
}

#[async_trait]
impl gen::RequiredParameters for Parameters {
    async fn query(&self, count: i32, active: bool, label: String) -> Result<gen::Inputs> {
        Ok(Self::inputs(count, active, label))
    }

    async fn header(&self, count: i32, active: bool, label: String) -> Result<gen::Inputs> {
        Ok(Self::inputs(count, active, label))
    }

    async fn cookie(&self, count: i32, active: bool, label: String) -> Result<gen::Inputs> {
        Ok(Self::inputs(count, active, label))
    }

    async fn optional(
        &self,
        count: Option<i32>,
        active: Option<bool>,
        label: Option<String>,
    ) -> Result<gen::Inputs> {
        let mut inputs = Self::inputs(
            count.unwrap_or_default(),
            active.unwrap_or_default(),
            label.clone().unwrap_or_default(),
        );
        inputs.note = label;
        Ok(inputs)
    }

    async fn stream(
        &self,
        req: xidl_rust_axum::Request<gen::RequiredParametersStreamRequest>,
    ) -> Result<xidl_rust_axum::stream::SseStream<gen::Inputs>> {
        let value = Self::inputs(req.data.count, req.data.active, req.data.label);
        Ok(xidl_rust_axum::stream::boxed_sse(
            futures_util::stream::once(async { Ok(value) }),
        ))
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    if let Some(base) = args.next() {
        let client = gen::RequiredParametersClient::new(base);
        for value in [
            client.query(0, false, String::new()).await?,
            client.header(0, false, String::new()).await?,
            client.cookie(0, false, String::new()).await?,
            client.optional(None, None, None).await?,
        ] {
            assert_eq!(value, Parameters::inputs(0, false, String::new()));
        }
        println!("PASS: explicit defaults and absent optional values");
        return Ok(());
    }
    let port = std::env::var("PORT")?;
    xidl_rust_axum::Server::builder()
        .with_service(gen::RequiredParametersServer::new(Parameters))
        .serve(format!("127.0.0.1:{port}"))
        .await?;
    Ok(())
}
