use std::collections::HashMap;

use tokio::sync::Mutex;

include!(concat!(env!("OUT_DIR"), "/http_server.rs"));

pub struct SimpleHttpServer {
    host: Mutex<String>,
    server_name: Mutex<String>,
    user_info: Mutex<HashMap<u64, UserInfo>>,
}

impl Default for SimpleHttpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleHttpServer {
    pub fn new() -> Self {
        Self {
            host: Mutex::new("localhost".to_string()),
            server_name: Mutex::new("http_server".to_string()),
            user_info: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl HttpServer for SimpleHttpServer {
    async fn host(
        &self,
        _req: xidl_rust_axum::Request<()>,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(self.host.lock().await.clone())
    }

    async fn set_host(
        &self,
        req: xidl_rust_axum::Request<HttpServerSetHostRequest>,
    ) -> Result<(), xidl_rust_axum::Error> {
        let req = req.into_inner();
        *self.host.lock().await = req.host;
        Ok(())
    }

    async fn port(&self, req: xidl_rust_axum::Request<()>) -> Result<u16, xidl_rust_axum::Error> {
        Ok(8081)
    }

    async fn get_server_name(
        &self,
        req: xidl_rust_axum::Request<()>,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(self.server_name.lock().await.clone())
    }

    async fn set_server_name(
        &self,
        req: xidl_rust_axum::Request<HttpServerSetServerNameRequest>,
    ) -> Result<(), xidl_rust_axum::Error> {
        let req = req.into_inner();
        *self.server_name.lock().await = req.name;
        Ok(())
    }

    async fn get_user_info(
        &self,
        req: xidl_rust_axum::Request<HttpServerGetUserInfoRequest>,
    ) -> Result<UserInfo, xidl_rust_axum::Error> {
        let req = req.into_inner();
        let user_info = self.user_info.lock().await;
        let user_info = user_info.get(&req.id);
        if let Some(user_info) = user_info {
            return Ok(user_info.clone());
        }

        Err(xidl_rust_axum::Error::not_found())
    }
}
