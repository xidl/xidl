use async_trait::async_trait;

mod generated {
    include!("../{{MODULE_NAME}}.rs");
}

struct MyAccounts;

#[async_trait]
impl generated::Accounts for MyAccounts {
    async fn participant<'a>(&'a self) -> Result<generated::Participant, xidl_rust_axum::Error> {
        Ok(generated::Participant {
            base: generated::UserRef {
                id: "u-1".into(),
                avatar_url: None,
            },
            role: "admin".into(),
        })
    }

    async fn echo<'a>(
        &'a self,
        participant: generated::Participant,
    ) -> Result<generated::Participant, xidl_rust_axum::Error> {
        Ok(participant)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = generated::AccountsServer::new(MyAccounts);
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    xidl_rust_axum::Server::builder()
        .with_service(service)
        .serve(&format!("127.0.0.1:{port}"))
        .await?;
    Ok(())
}
