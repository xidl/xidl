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

    async fn letters(&self, letters: Vec<char>) -> Result<Vec<char>> {
        Ok(letters)
    }

    async fn letter_aliases(
        &self,
        letters: gen::transport::Letters,
    ) -> Result<gen::transport::Letters> {
        Ok(letters)
    }

    async fn wide_letters(
        &self,
        letters: Vec<gen::transport::WideLetter>,
    ) -> Result<Vec<gen::transport::WideLetter>> {
        Ok(letters)
    }

    async fn initial(&self, letter: char) -> Result<char> {
        Ok(letter)
    }

    async fn cookie_letters(&self, letters: Vec<char>) -> Result<Vec<char>> {
        Ok(letters)
    }

    async fn numbers(
        &self,
        count: i32,
        ratio: f64,
        units: u32,
        threshold: Option<f64>,
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
        let client = gen::CollectionInputsClient::new(base.clone());
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
        let numbers = client.numbers(2, -50.0, 2, Some(0.0)).await?;
        assert_eq!(numbers.count, 2);
        assert_eq!(numbers.ratio, -50.0);
        assert_eq!(numbers.units, 2);
        assert_eq!(numbers.threshold, Some(0.0));
        let numbers = client.numbers(0, 0.0, 0, None).await?;
        assert_eq!(numbers.threshold, None);
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
        for letters in [vec![], vec!['a'], vec!['a', 'b', 'a']] {
            assert_eq!(client.letters(letters.clone()).await?, letters);
            assert_eq!(client.letter_aliases(letters.clone()).await?, letters);
            assert_eq!(client.wide_letters(letters.clone()).await?, letters);
        }
        assert_eq!(client.initial('A').await?, 'A');
        let cookie_letters = vec![',', ' ', '雪'];
        assert_eq!(
            client.cookie_letters(cookie_letters.clone()).await?,
            cookie_letters
        );
        for letter in [',', ' '] {
            let error = client.letters(vec![letter]).await.unwrap_err();
            assert_eq!(error.code, 400);
            assert!(error
                .message
                .contains("unrepresentable simple header collection element: letters"));
        }
        let raw = xidl_rust_axum::reqwest::Client::new();
        for value in ["", "ab", "a,,b", "\"a\""] {
            let response = raw
                .get(format!("{base}/letters"))
                .header("letters", value)
                .send()
                .await?;
            assert_eq!(
                response.status().as_u16(),
                400,
                "invalid character header {value:?}"
            );
        }
        for value in ["", "ab", "\"a\""] {
            let response = raw
                .get(format!("{base}/initial"))
                .header("letter", value)
                .send()
                .await?;
            assert_eq!(
                response.status().as_u16(),
                400,
                "invalid scalar character header {value:?}"
            );
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
