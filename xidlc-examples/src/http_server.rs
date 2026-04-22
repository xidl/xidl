#![allow(deprecated)]

use std::collections::HashMap;

use tokio::sync::Mutex;

include!(concat!(env!("OUT_DIR"), "/http_server.rs"));

pub struct SimpleHttpServer {
    host: Mutex<String>,
    server_name: Mutex<String>,
    user_info: Mutex<HashMap<u64, UserInfo>>,
    key_store: Mutex<HashMap<String, String>>,
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
            key_store: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl HttpServer for SimpleHttpServer {
    async fn get_attribute_host(&self) -> Result<String, xidl_rust_axum::Error> {
        Ok(self.host.lock().await.clone())
    }

    async fn set_attribute_host(&self, host: String) -> Result<(), xidl_rust_axum::Error> {
        *self.host.lock().await = host;
        Ok(())
    }

    async fn get_attribute_port(&self) -> Result<u16, xidl_rust_axum::Error> {
        Ok(8081)
    }

    async fn get_server_name(&self) -> Result<String, xidl_rust_axum::Error> {
        Ok(self.server_name.lock().await.clone())
    }

    async fn set_server_name(&self, name: String) -> Result<(), xidl_rust_axum::Error> {
        *self.server_name.lock().await = name;
        Ok(())
    }

    async fn get_user_info(&self, id: u64) -> Result<UserInfo, xidl_rust_axum::Error> {
        let user_info = self.user_info.lock().await;
        let user_info = user_info.get(&id);
        if let Some(user_info) = user_info {
            return Ok(user_info.clone());
        }

        Err(xidl_rust_axum::Error::not_found())
    }

    async fn query_user_info(&self, id: u64) -> Result<UserInfo, xidl_rust_axum::Error> {
        self.get_user_info(id).await
    }

    async fn post_user_info(&self, id: u64, info: UserInfo) -> Result<(), xidl_rust_axum::Error> {
        self.user_info.lock().await.insert(id, info);
        Ok(())
    }

    async fn put_key_value(
        &self,
        key: String,
        value: String,
        ttl: u64,
    ) -> Result<(), xidl_rust_axum::Error> {
        println!("insert {key}: {value} at {ttl}");
        self.key_store.lock().await.insert(key, value);
        Ok(())
    }

    async fn delete_key(&self, key: String) -> Result<(), xidl_rust_axum::Error> {
        self.key_store.lock().await.remove(&key);
        Ok(())
    }

    async fn patch_key(&self, key: String, value: String) -> Result<(), xidl_rust_axum::Error> {
        self.key_store.lock().await.insert(key, value);
        Ok(())
    }

    async fn is_key_exists(&self, key_alias: String) -> Result<(), xidl_rust_axum::Error> {
        if self.key_store.lock().await.contains_key(&key_alias) {
            return Ok(());
        }
        Err(xidl_rust_axum::Error::not_found())
    }

    async fn get_key_options(
        &self,
        key: String,
    ) -> Result<HttpServerGetKeyOptionsResponse, xidl_rust_axum::Error> {
        Ok(HttpServerGetKeyOptionsResponse {
            exists: self.key_store.lock().await.contains_key(&key),
        })
    }

    async fn get_key_1(
        &self,
        key: String,
    ) -> Result<HttpServerGetKey1Response, xidl_rust_axum::Error> {
        if let Some(value) = self.key_store.lock().await.get(&key) {
            return Ok(HttpServerGetKey1Response {
                value: value.clone(),
            });
        }

        Err(xidl_rust_axum::Error::not_found())
    }

    async fn get_key_2(
        &self,
        key: String,
    ) -> Result<HttpServerGetKey2Response, xidl_rust_axum::Error> {
        let response = self.get_key_1(key).await?;
        Ok(HttpServerGetKey2Response {
            value: response.value,
        })
    }

    async fn get_key_3(
        &self,
        key: String,
    ) -> Result<HttpServerGetKey3Response, xidl_rust_axum::Error> {
        let response = self.get_key_1(key).await?;
        Ok(HttpServerGetKey3Response {
            value: response.value,
        })
    }

    async fn get_key_4(
        &self,
        key: String,
    ) -> Result<HttpServerGetKey4Response, xidl_rust_axum::Error> {
        let response = self.get_key_1(key).await?;
        Ok(HttpServerGetKey4Response {
            value: response.value,
        })
    }

    async fn login(
        &self,
        xidl_auth: xidl_rust_axum::auth::basic::BasicAuth,
    ) -> Result<HttpServerLoginResponse, xidl_rust_axum::Error> {
        println!("login: {:?}", xidl_auth);
        match xidl_auth.password {
            None => {
                return Err(xidl_rust_axum::Error::unauthorized());
            }
            Some(pass) if pass.is_empty() => {
                return Err(xidl_rust_axum::Error::unauthorized());
            }
            _ => {}
        }

        Ok(HttpServerLoginResponse {
            session_id: "simple_session_id".to_string(),
        })
    }

    async fn login_realm(
        &self,
        xidl_auth: xidl_rust_axum::auth::basic::BasicAuth,
    ) -> Result<HttpServerLoginRealmResponse, xidl_rust_axum::Error> {
        println!("login: {:?}", xidl_auth);
        match xidl_auth.password {
            None => {
                return Err(xidl_rust_axum::Error::unauthorized());
            }
            Some(pass) if pass.is_empty() => {
                return Err(xidl_rust_axum::Error::unauthorized());
            }
            _ => {}
        }

        Ok(HttpServerLoginRealmResponse {
            session_id: "simple_session_id".to_string(),
        })
    }

    async fn is_logined(&self, session_id: String) -> Result<bool, xidl_rust_axum::Error> {
        println!("is_logined: {}", session_id);
        Ok(!session_id.is_empty())
    }

    async fn login_bearer(
        &self,
        xidl_auth: xidl_rust_axum::auth::bearer::BearerAuth,
    ) -> Result<(), xidl_rust_axum::Error> {
        if xidl_auth.token.is_empty() {
            return Err(xidl_rust_axum::Error::unauthorized());
        }
        Ok(())
    }
    async fn get_timestamp(&self) -> ::xidl_rust_axum::Result<Timestamp> {
        todo!()
    }
    async fn is_admin(&self, info: AdminInfo) -> ::xidl_rust_axum::Result<()> {
        todo!()
    }
}
