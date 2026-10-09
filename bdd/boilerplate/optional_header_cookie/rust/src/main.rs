use async_trait::async_trait;

pub mod test {
    pub use crate::gen::test::*;
}

mod gen {
    include!("../{{MODULE_NAME}}.rs");
}

struct MyOptionalService;

#[async_trait]
impl test::OptionalTestService for MyOptionalService {
    async fn test_optional(
        &self,
        id: String,
        opt_header: Option<String>,
        opt_cookie: Option<String>,
        multi_header: Option<Vec<String>>,
        multi_cookie: Option<Vec<String>>,
    ) -> Result<test::EchoResult, xidl_rust_axum::Error> {
        Ok(test::EchoResult {
            id,
            header_echo: opt_header.unwrap_or_else(|| "NONE".to_string()),
            cookie_echo: opt_cookie.unwrap_or_else(|| "NONE".to_string()),
            multi_header_echo: multi_header.unwrap_or_default(),
            multi_cookie_echo: multi_cookie.unwrap_or_default(),
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let addr = format!("127.0.0.1:{}", port);
    let svc = test::OptionalTestServiceServer::new(MyOptionalService);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let local_addr = listener.local_addr()?;

    let server_handle = tokio::spawn(async move {
        let server = xidl_rust_axum::Server::builder().with_service(svc);
        let _ = server.serve_with_listener(listener).await;
    });

    // Verify generated client against running server
    let client = test::OptionalTestServiceClient::new(format!("http://{}", local_addr));

    // Call with all None
    let res = client
        .test_optional("c1".to_string(), None, None, None, None)
        .await?;
    assert_eq!(res.header_echo, "NONE");
    assert_eq!(res.cookie_echo, "NONE");
    assert!(res.multi_header_echo.is_empty());
    assert!(res.multi_cookie_echo.is_empty());

    // Call with Some values
    let res = client
        .test_optional(
            "c2".to_string(),
            Some("client-head".to_string()),
            Some("client-cookie".to_string()),
            Some(vec!["h1".to_string(), "h2".to_string()]),
            Some(vec!["c1".to_string(), "c2".to_string()]),
        )
        .await?;
    assert_eq!(res.header_echo, "client-head");
    assert_eq!(res.cookie_echo, "client-cookie");
    assert_eq!(res.multi_header_echo, vec!["h1", "h2"]);
    assert_eq!(res.multi_cookie_echo, vec!["c1", "c2"]);

    println!("Rust server and client self-test started on {}", addr);
    server_handle.await?;
    Ok(())
}
