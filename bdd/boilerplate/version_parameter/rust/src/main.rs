use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use percent_encoding::percent_decode_str;
use std::{env, time::Duration};
use tokio::{net::TcpListener, sync::mpsc, time::timeout};
use xidl_rust_axum::Service;

// gen.rs is produced from version_parameter.idl by the BDD generation step.
pub mod gen;
mod service;
use service::Store;

struct Observed {
    uri: String,
    request: axum::http::HeaderMap,
    response: axum::http::HeaderMap,
    status: u16,
}

async fn capture(
    State(tx): State<mpsc::UnboundedSender<Observed>>,
    req: Request,
    next: Next,
) -> Response {
    let uri = req.uri().to_string();
    let request = req.headers().clone();
    let response = next.run(req).await;
    tx.send(Observed {
        uri,
        request,
        response: response.headers().clone(),
        status: response.status().as_u16(),
    })
    .unwrap();
    response
}

struct Contract {
    location: String,
    name: String,
}
impl Contract {
    fn response(&self, observed: &Observed, value: &str) {
        if self.location == "header" {
            assert_eq!(observed.response.get(&self.name).unwrap(), value);
        } else {
            assert!(!observed.response.contains_key(&self.name));
            assert!(!observed.response.contains_key("set-cookie"));
        }
    }
    async fn observe(
        &self,
        rx: &mut mpsc::UnboundedReceiver<Observed>,
        value: &str,
        status: u16,
    ) -> Observed {
        let observed = timeout(Duration::from_secs(5), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(observed.status, status, "{}", observed.uri);
        let values = match self.location.as_str() {
            "header" => observed
                .request
                .get_all(&self.name)
                .iter()
                .map(|v| v.to_str().unwrap().to_string())
                .collect::<Vec<_>>(),
            "query" => url::Url::parse(&format!("http://localhost{}", observed.uri))
                .unwrap()
                .query_pairs()
                .filter(|(key, _)| key == self.name.as_str())
                .map(|(_, value)| value.into_owned())
                .collect(),
            "cookie" => observed
                .request
                .get_all("cookie")
                .iter()
                .flat_map(|v| v.to_str().unwrap().split(';'))
                .filter_map(|v| v.trim().split_once('='))
                .filter(|(key, _)| *key == self.name)
                .map(|(_, value)| {
                    percent_decode_str(value)
                        .decode_utf8()
                        .unwrap()
                        .into_owned()
                })
                .collect(),
            _ => unreachable!(),
        };
        assert_eq!(values, [value], "{}", observed.uri);
        self.response(&observed, value);
        observed
    }
}

macro_rules! exercise {
    ($version:ident, $value:expr, $contract:expr) => {{
        let expected = $value;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let router = gen::$version::business::StoreServer::new(Store)
            .into_router()
            .merge(gen::$version::business::HealthServer::new(Store).into_router())
            .merge(gen::$version::business::DuplexShapesServer::new(Store).into_router())
            .layer(axum::middleware::from_fn_with_state(tx, capture));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    let _ = stopped.await;
                })
                .await
                .unwrap();
        });
        let mut defaults = axum::http::HeaderMap::new();
        defaults.insert("x-version", "caller-old-version".parse().unwrap());
        let http = xidl_rust_axum::reqwest::Client::builder()
            .default_headers(defaults)
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let client = gen::$version::business::StoreClient::with_http(&base, http.clone());
        assert_eq!(gen::$version::business::StoreClient::API_VERSION, expected);
        assert_eq!(gen::$version::business::HealthClient::API_VERSION, expected);
        for _ in 0..2 {
            assert_eq!(
                client
                    .get(7, "a +/%".into(), "dark".into())
                    .await
                    .unwrap()
                    .id,
                "7"
            );
            let observed = $contract.observe(&mut rx, expected, 200).await;
            assert!(observed
                .request
                .get("cookie")
                .unwrap()
                .to_str()
                .unwrap()
                .contains("theme=dark"));
        }
        client.remove(7).await.unwrap();
        $contract.observe(&mut rx, expected, 204).await;
        assert!(matches!(
            client.fail(1).await,
            Err(gen::$version::business::StoreFailError::Conflict(_))
        ));
        $contract.observe(&mut rx, expected, 409).await;
        assert!(matches!(
            client.fail(2).await,
            Err(gen::$version::business::StoreFailError::Framework(_))
        ));
        $contract.observe(&mut rx, expected, 500).await;
        let health = gen::$version::business::HealthClient::with_http(&base, http.clone());
        assert_eq!(health.health().await.unwrap(), "healthy");
        $contract.observe(&mut rx, expected, 200).await;
        let mut chat = timeout(
            Duration::from_secs(5),
            client.chat("team +/%".into(), String::new()),
        )
        .await
        .expect("WebSocket handshake deadline")
        .unwrap();
        $contract.observe(&mut rx, expected, 101).await;
        chat.write(gen::$version::business::StoreChatRequest {
            text: "hello".into(),
        })
        .await
        .unwrap();
        let reply = timeout(Duration::from_secs(5), chat.read())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(reply.reply, "hello:ack");
        chat.close();
        let closed = timeout(Duration::from_secs(5), chat.read()).await.unwrap();
        assert!(
            closed.is_none(),
            "unexpected item after close: {:?}",
            closed.map(|value| value.map(|_| ()))
        );
        let shapes = gen::$version::business::DuplexShapesClient::with_http(&base, http.clone());
        let mut scalar = timeout(Duration::from_secs(5), shapes.scalar("hello".into()))
            .await
            .expect("scalar WebSocket handshake deadline")
            .unwrap();
        $contract.observe(&mut rx, expected, 101).await;
        scalar
            .write(gen::$version::business::DuplexShapesScalarRequest {
                text: "hello".into(),
            })
            .await
            .unwrap();
        assert_eq!(
            timeout(Duration::from_secs(5), scalar.read())
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
            "hello:scalar"
        );
        scalar.close();
        assert!(timeout(Duration::from_secs(5), scalar.read())
            .await
            .unwrap()
            .is_none());

        let mut model = timeout(Duration::from_secs(5), shapes.model("object".into()))
            .await
            .expect("model WebSocket handshake deadline")
            .unwrap();
        $contract.observe(&mut rx, expected, 101).await;
        model
            .write(gen::$version::business::DuplexShapesModelRequest {
                id: "object".into(),
            })
            .await
            .unwrap();
        assert_eq!(
            timeout(Duration::from_secs(5), model.read())
                .await
                .unwrap()
                .unwrap()
                .unwrap()
                .id,
            "object"
        );
        model.close();
        assert!(timeout(Duration::from_secs(5), model.read())
            .await
            .unwrap()
            .is_none());

        let mut sink = timeout(Duration::from_secs(5), shapes.sink("stored".into()))
            .await
            .expect("sink WebSocket handshake deadline")
            .unwrap();
        $contract.observe(&mut rx, expected, 101).await;
        sink.write(gen::$version::business::DuplexShapesSinkRequest {
            text: "stored".into(),
        })
        .await
        .unwrap();
        sink.close();
        assert!(timeout(Duration::from_secs(5), sink.read())
            .await
            .unwrap()
            .is_none());

        let mut source = timeout(Duration::from_secs(5), shapes.source())
            .await
            .expect("source WebSocket handshake deadline")
            .unwrap();
        $contract.observe(&mut rx, expected, 101).await;
        assert_eq!(
            timeout(Duration::from_secs(5), source.read())
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
            "announcement"
        );
        source.close();
        assert!(timeout(Duration::from_secs(5), source.read())
            .await
            .unwrap()
            .is_none());
        let http = xidl_rust_axum::reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let other_version = env::var("VERSION_VALUE").unwrap();
        for requested in [
            None,
            Some(if expected == "2026-04" {
                other_version.as_str()
            } else {
                "2026-04"
            }),
            Some("unknown"),
        ] {
            let mut url = url::Url::parse(&format!("{base}/health")).unwrap();
            if $contract.location == "query" {
                if let Some(value) = requested {
                    url.query_pairs_mut().append_pair(&$contract.name, value);
                }
            }
            let mut req = http.get(url);
            if let Some(value) = requested {
                req = match $contract.location.as_str() {
                    "header" => req.header(&$contract.name, value),
                    "query" => req,
                    "cookie" => req.header(
                        "Cookie",
                        format!(
                            "{}={}",
                            $contract.name,
                            percent_encoding::utf8_percent_encode(
                                value,
                                percent_encoding::NON_ALPHANUMERIC
                            )
                        ),
                    ),
                    _ => unreachable!(),
                };
            }
            let response = req.send().await.unwrap();
            assert_eq!(response.status(), 200);
            let observed = timeout(Duration::from_secs(5), rx.recv())
                .await
                .unwrap()
                .unwrap();
            $contract.response(&observed, expected);
        }
        for path in ["/items/not-a-number?search=x", "/items/%25ZZ?search=x"] {
            let response = http.get(format!("{base}{path}")).send().await.unwrap();
            assert_eq!(response.status(), 400);
            let observed = timeout(Duration::from_secs(5), rx.recv())
                .await
                .unwrap()
                .unwrap();
            $contract.response(&observed, expected);
        }
        stop.send(()).unwrap();
        timeout(Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap();
    }};
}

#[tokio::main]
async fn main() {
    let contract = Contract {
        location: env::var("VERSION_LOCATION").unwrap(),
        name: env::var("VERSION_NAME").unwrap(),
    };
    exercise!(v1, "2026-04", contract);
    let value = env::var("VERSION_VALUE").unwrap();
    exercise!(v2, value.as_str(), contract);
    println!("Rust generated version parameter HTTP and WebSocket workflows passed");
}
