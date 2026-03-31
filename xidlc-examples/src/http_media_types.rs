include!(concat!(env!("OUT_DIR"), "/http_media_types.rs"));

pub struct HttpMediaTypesService;

#[async_trait::async_trait]
impl HttpMediaTypesApi for HttpMediaTypesService {
    async fn submit_profile(
        &self,
        req: xidl_rust_axum::Request<HttpMediaTypesApiSubmitProfileRequest>,
    ) -> Result<HttpMediaTypesApiSubmitProfileResponse, xidl_rust_axum::Error> {
        let data = req.into_inner();
        Ok(HttpMediaTypesApiSubmitProfileResponse {
            r#return: format!("{}:{}", data.name, data.age),
            normalized_name: data.name.to_ascii_uppercase(),
        })
    }

    async fn get_msgpack_user(
        &self,
        req: xidl_rust_axum::Request<HttpMediaTypesApiGetMsgpackUserRequest>,
    ) -> Result<HttpMediaTypesApiGetMsgpackUserResponse, xidl_rust_axum::Error> {
        let data = req.into_inner();
        Ok(HttpMediaTypesApiGetMsgpackUserResponse {
            r#return: format!("user:{}", data.user_id),
            score: 95,
        })
    }
}
