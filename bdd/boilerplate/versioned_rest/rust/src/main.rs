use async_trait::async_trait;
use std::env;

mod gen {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/{{MODULE_NAME}}.rs"));
}

struct Items;

#[async_trait]
impl gen::v1::Items for Items {
    async fn get(&self, _id: String) -> ::xidl_rust_axum::Result<gen::v1::Item> {
        Ok(gen::v1::Item {
            name: "item".to_string(),
        })
    }

    async fn legacy(&self) -> ::xidl_rust_axum::Result<String> {
        Ok("legacy".to_string())
    }
}

#[async_trait]
impl gen::v2::Items for Items {
    async fn get(&self, _id: String) -> ::xidl_rust_axum::Result<gen::v2::Item> {
        Ok(gen::v2::Item {
            name: "item".to_string(),
            note: Some("v2".to_string()),
        })
    }

    async fn list(&self) -> ::xidl_rust_axum::Result<Vec<gen::v2::Item>> {
        Ok(vec![gen::v2::Item {
            name: "item".to_string(),
            note: Some("v2".to_string()),
        }])
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port_v1 = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let port_v2 = env::var("PORT_V2").unwrap_or_else(|_| "8081".to_string());
    let addr_v1 = format!("127.0.0.1:{port_v1}");
    let addr_v2 = format!("127.0.0.1:{port_v2}");
    println!("Versioned rust server starting on {addr_v1} (v1) and {addr_v2} (v2)");

    let v1 = xidl_rust_axum::Server::builder()
        .with_service(gen::v1::ItemsServer::new(Items))
        .serve(addr_v1);
    let v2 = xidl_rust_axum::Server::builder()
        .with_service(gen::v2::ItemsServer::new(Items))
        .serve(addr_v2);
    tokio::try_join!(v1, v2)?;
    Ok(())
}
