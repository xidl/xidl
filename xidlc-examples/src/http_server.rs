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
    async fn get_attribute_host(
        &self,
        _req: xidl_rust_axum::Request<()>,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(self.host.lock().await.clone())
    }

    async fn set_attribute_host(
        &self,
        req: xidl_rust_axum::Request<HttpServerSetAttributeHostRequest>,
    ) -> Result<(), xidl_rust_axum::Error> {
        let req = req.into_inner();
        *self.host.lock().await = req.host;
        Ok(())
    }

    async fn get_attribute_port(
        &self,
        req: xidl_rust_axum::Request<()>,
    ) -> Result<u16, xidl_rust_axum::Error> {
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

    async fn query_user_info(
        &self,
        req: xidl_rust_axum::Request<HttpServerQueryUserInfoRequest>,
    ) -> Result<UserInfo, xidl_rust_axum::Error> {
        self.get_user_info(xidl_rust_axum::Request::new(
            req.headers().clone(),
            HttpServerGetUserInfoRequest {
                id: req.into_inner().id,
            },
        ))
        .await
    }

    async fn post_user_info(
        &self,
        req: xidl_rust_axum::Request<HttpServerPostUserInfoRequest>,
    ) -> Result<(), xidl_rust_axum::Error> {
        let req = req.into_inner();
        self.user_info.lock().await.insert(req.id, req.info);
        Ok(())
    }

    async fn put_key_value(
        &self,
        req: xidl_rust_axum::Request<HttpServerPutKeyValueRequest>,
    ) -> Result<(), xidl_rust_axum::Error> {
        let inner = req.into_inner();
        println!("insert {}: {} at {}", inner.key, inner.value, inner.ttl);
        self.key_store.lock().await.insert(inner.key, inner.value);
        Ok(())
    }

    async fn delete_key(
        &self,
        req: xidl_rust_axum::Request<HttpServerDeleteKeyRequest>,
    ) -> Result<(), xidl_rust_axum::Error> {
        self.key_store.lock().await.remove(&req.into_inner().key);
        Ok(())
    }

    async fn patch_key(
        &self,
        req: xidl_rust_axum::Request<HttpServerPatchKeyRequest>,
    ) -> Result<(), xidl_rust_axum::Error> {
        let rq = req.into_inner();
        self.key_store.lock().await.insert(rq.key, rq.value);
        Ok(())
    }

    async fn is_key_exists(
        &self,
        req: xidl_rust_axum::Request<HttpServerIsKeyExistsRequest>,
    ) -> Result<(), xidl_rust_axum::Error> {
        let req = req.into_inner();
        if self.key_store.lock().await.contains_key(&req.key_alias) {
            return Ok(());
        }
        Err(xidl_rust_axum::Error::not_found())
    }

    async fn get_key_options(
        &self,
        req: xidl_rust_axum::Request<HttpServerGetKeyOptionsRequest>,
    ) -> Result<bool, xidl_rust_axum::Error> {
        let req = req.into_inner();
        Ok(self.key_store.lock().await.contains_key(&req.key))
    }

    async fn get_key_1(
        &self,
        req: xidl_rust_axum::Request<HttpServerGetKey1Request>,
    ) -> Result<String, xidl_rust_axum::Error> {
        let req = req.into_inner();
        if let Some(value) = self.key_store.lock().await.get(&req.key) {
            return Ok(value.clone());
        }

        Err(xidl_rust_axum::Error::not_found())
    }

    async fn get_key_2(
        &self,
        req: xidl_rust_axum::Request<HttpServerGetKey2Request>,
    ) -> Result<String, xidl_rust_axum::Error> {
        self.get_key_1(xidl_rust_axum::Request {
            headers: req.headers,
            data: HttpServerGetKey1Request { key: req.data.key },
        })
        .await
    }

    async fn get_key_3(
        &self,
        req: xidl_rust_axum::Request<HttpServerGetKey3Request>,
    ) -> Result<String, xidl_rust_axum::Error> {
        self.get_key_1(xidl_rust_axum::Request {
            headers: req.headers,
            data: HttpServerGetKey1Request { key: req.data.key },
        })
        .await
    }

    async fn get_key_4(
        &self,
        req: xidl_rust_axum::Request<HttpServerGetKey4Request>,
    ) -> Result<String, xidl_rust_axum::Error> {
        self.get_key_1(xidl_rust_axum::Request {
            headers: req.headers,
            data: HttpServerGetKey1Request { key: req.data.key },
        })
        .await
    }

    async fn login(
        &self,
        req: xidl_rust_axum::Request<HttpServerLoginRequest>,
    ) -> Result<String, xidl_rust_axum::Error> {
        let auth = req.into_inner().xidl_auth;
        println!("login: {:?}", auth);
        match auth.password {
            None => {
                return Err(xidl_rust_axum::Error::unauthorized());
            }
            Some(pass) if pass.is_empty() => {
                return Err(xidl_rust_axum::Error::unauthorized());
            }
            _ => {}
        }

        Ok("simple_session_id".to_string())
    }

    async fn login_realm(
        &self,
        req: xidl_rust_axum::Request<HttpServerLoginRealmRequest>,
    ) -> Result<String, xidl_rust_axum::Error> {
        let auth = req.into_inner().xidl_auth;
        println!("login: {:?}", auth);
        match auth.password {
            None => {
                return Err(xidl_rust_axum::Error::unauthorized());
            }
            Some(pass) if pass.is_empty() => {
                return Err(xidl_rust_axum::Error::unauthorized());
            }
            _ => {}
        }

        Ok("simple_session_id".to_string())
    }

    async fn is_logined(
        &self,
        req: xidl_rust_axum::Request<HttpServerIsLoginedRequest>,
    ) -> Result<bool, xidl_rust_axum::Error> {
        let req = req.into_inner();

        println!("is_logined: {}", req.session_id);
        Ok(!req.session_id.is_empty())
    }

    async fn login_bearer(
        &self,
        req: xidl_rust_axum::Request<HttpServerLoginBearerRequest>,
    ) -> Result<(), xidl_rust_axum::Error> {
        let auth = req.into_inner().xidl_auth;
        if auth.token.is_empty() {
            return Err(xidl_rust_axum::Error::unauthorized());
        }
        Ok(())
    }
}
