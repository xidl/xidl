use async_trait::async_trait;
use xidl_rust_axum::Result;

pub mod gen {
    include!("../{{MODULE_NAME}}.rs");
}

struct Inputs;

#[async_trait]
impl gen::CollectionInputs for Inputs {
    async fn read(
        &self,
        values: Vec<i32>,
        tags: Vec<String>,
        names: Vec<String>,
        labels: Option<Vec<String>>,
    ) -> Result<gen::Collections> {
        Ok(gen::Collections {
            values,
            tags,
            names,
            labels,
        })
    }

    async fn search(&self, term: Option<String>) -> Result<gen::Search> {
        Ok(gen::Search { term })
    }

    async fn numbers(
        &self,
        count: i32,
        ratio: f64,
        units: u32,
        threshold: Option<i32>,
    ) -> Result<gen::Numbers> {
        Ok(gen::Numbers {
            count,
            ratio,
            units,
            threshold,
        })
    }

    async fn stream(
        &self,
        req: xidl_rust_axum::Request<gen::CollectionInputsStreamRequest>,
    ) -> Result<xidl_rust_axum::stream::SseStream<gen::StreamValues>> {
        let values = gen::StreamValues {
            values: req.data.values,
        };
        Ok(xidl_rust_axum::stream::boxed_sse(
            futures_util::stream::once(async { Ok(values) }),
        ))
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    if let Some(base) = std::env::args().nth(1) {
        let client = gen::CollectionInputsClient::new(base);
        for labels in [None, Some(vec![])] {
            assert_eq!(
                client.read(vec![], vec![], vec![], labels).await?,
                gen::Collections {
                    values: vec![],
                    tags: vec![],
                    names: vec![],
                    labels: None,
                }
            );
        }
        for labels in [vec![String::new()], vec!["a&b".into(), "雪".into()]] {
            let value = client
                .read(
                    vec![0, -2, 0],
                    vec!["first".into(), "second".into()],
                    labels.clone(),
                    Some(labels.clone()),
                )
                .await?;
            assert_eq!(value.values, vec![0, -2, 0]);
            assert_eq!(value.tags, vec!["first", "second"]);
            assert_eq!(value.names, labels);
            assert_eq!(value.labels, Some(labels));
        }
        for term in [None, Some(String::new()), Some("a&b 雪".into())] {
            assert_eq!(client.search(term.clone()).await?.term, term);
        }
        for threshold in [None, Some(0), Some(-2)] {
            assert_eq!(
                client.numbers(0, 0.0, 0, threshold).await?,
                gen::Numbers {
                    count: 0,
                    ratio: 0.0,
                    units: 0,
                    threshold,
                }
            );
        }
        assert_eq!(client.numbers(-2, 0.5, 2, None).await?.ratio, 0.5);
        for tag in ["one,two", " leading", "trailing "] {
            let error = client
                .read(vec![], vec![tag.into()], vec![], None)
                .await
                .unwrap_err();
            assert_eq!(error.code, 400);
            assert!(error
                .message
                .contains("unrepresentable simple header collection element: tags"));
        }
        println!("PASS: collection SDK wire occurrences");
        return Ok(());
    }
    xidl_rust_axum::Server::builder()
        .with_service(gen::CollectionInputsServer::new(Inputs))
        .serve(format!("127.0.0.1:{}", std::env::var("PORT")?))
        .await?;
    Ok(())
}
