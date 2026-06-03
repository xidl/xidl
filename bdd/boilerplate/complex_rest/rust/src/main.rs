use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::env;

mod gen {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/complex_rest.rs"));
}

struct MyUserService {
    users: Arc<Mutex<HashMap<u32, gen::User>>>,
}

#[async_trait]
impl gen::UserService for MyUserService {
    async fn get_user<'a>(&'a self, id: u32) -> Result<gen::User, xidl_rust_axum::Error> {
        let users = self.users.lock().unwrap();
        users.get(&id).cloned().ok_or(xidl_rust_axum::Error::not_found())
    }
    async fn create_user<'a>(&'a self, user: gen::User) -> Result<gen::User, xidl_rust_axum::Error> {
        let mut users = self.users.lock().unwrap();
        users.insert(user.id, user.clone());
        Ok(user)
    }
    async fn list_users<'a>(&'a self, _filter: String) -> Result<Vec<gen::User>, xidl_rust_axum::Error> {
        let users = self.users.lock().unwrap();
        Ok(users.values().cloned().collect())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("127.0.0.1:{}", port);
    let svc = gen::UserServiceServer::new(MyUserService {
        users: Arc::new(Mutex::new(HashMap::new())),
    });
    println!("Rust server starting on {}", addr);
    xidl_rust_axum::Server::builder()
        .with_service(svc)
        .serve(&addr)
        .await?;
    Ok(())
}
