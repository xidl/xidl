use async_trait::async_trait;
use xidl_rust_axum::Error;

mod generated {
    include!("../{{MODULE_NAME}}.rs");
}

struct ScopedService;

#[async_trait]
impl generated::left::nested::Scoped for ScopedService {
    async fn exchange<'a>(
        &'a self,
        payload: generated::left::Envelope,
    ) -> Result<generated::right::Item, Error> {
        Ok(generated::right::Item {
            label: format!("{}:{}", payload.items[0].name, payload.root.id),
        })
    }

    async fn local<'a>(
        &'a self,
        payload: generated::left::nested::Item,
    ) -> Result<generated::left::nested::Item, Error> {
        Ok(payload)
    }

    async fn root<'a>(&'a self, payload: generated::Root) -> Result<generated::Root, Error> {
        Ok(payload)
    }
    async fn failure<'a>(
        &'a self,
    ) -> Result<(), generated::left::nested::ScopedFailureError> {
        Err(generated::left::nested::ScopedFailureError::Failure(
            generated::errors::Failure { hint: generated::errors::Hint::Busy },
        ))
    }

    async fn choice<'a>(
        &'a self,
        payload: generated::left::Choice,
    ) -> Result<generated::left::Choice, Error> {
        Ok(payload)
    }

}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("PORT")?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move {
        xidl_rust_axum::Server::builder()
            .with_service(generated::left::nested::ScopedServer::new(ScopedService))
            .serve_with_listener(listener).await
    });
    let client = generated::left::nested::ScopedClient::new(format!("http://{address}"));
    let text = client.choice(generated::left::Choice::new_text("scoped".into())).await?;
    assert_eq!(text.as_text(), "scoped");
    let number = client.choice(generated::left::Choice::new_number(7)).await?;
    assert_eq!(*number.as_number(), 7);
    assert!(matches!(client.failure().await,
        Err(generated::left::nested::ScopedFailureError::Failure(
            generated::errors::Failure { hint: generated::errors::Hint::Busy }
        ))));
    server.abort();
    let _ = server.await;
    let service = generated::left::nested::ScopedServer::new(ScopedService);
    xidl_rust_axum::Server::builder()
        .with_service(service)
        .serve(&format!("127.0.0.1:{port}"))
        .await?;
    Ok(())
}
