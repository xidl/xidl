use async_trait::async_trait;
use std::env;

mod gen {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/rest_media_types.rs"));
}

struct MyRestMediaTypesService;

#[async_trait]
impl gen::RestMediaTypesApi for MyRestMediaTypesService {
    async fn submit_profile<'a>(
        &'a self,
        name: String,
        age: u32,
    ) -> Result<gen::RestMediaTypesApiSubmitProfileResponse, xidl_rust_axum::Error> {
        Ok(gen::RestMediaTypesApiSubmitProfileResponse {
            r#return: format!("{name}:{age}"),
            normalized_name: name.to_ascii_uppercase(),
        })
    }

    async fn get_msgpack_user<'a>(
        &'a self,
        user_id: String,
    ) -> Result<gen::RestMediaTypesApiGetMsgpackUserResponse, xidl_rust_axum::Error> {
        Ok(gen::RestMediaTypesApiGetMsgpackUserResponse {
            r#return: format!("user:{user_id}"),
            score: 95,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("127.0.0.1:{}", port);
    let svc = gen::RestMediaTypesApiServer::new(MyRestMediaTypesService);
    println!("Rust server starting on {}", addr);
    xidl_rust_axum::Server::builder()
        .with_service(svc)
        .serve(&addr)
        .await?;
    Ok(())
}
