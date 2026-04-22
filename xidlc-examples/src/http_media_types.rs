include!(concat!(env!("OUT_DIR"), "/http_media_types.rs"));

pub struct HttpMediaTypesService;

#[async_trait::async_trait]
impl HttpMediaTypesApi for HttpMediaTypesService {
    async fn submit_profile(
        &self,
        name: String,
        age: u32,
    ) -> Result<HttpMediaTypesApiSubmitProfileResponse, xidl_rust_axum::Error> {
        Ok(HttpMediaTypesApiSubmitProfileResponse {
            r#return: format!("{name}:{age}"),
            normalized_name: name.to_ascii_uppercase(),
        })
    }

    async fn get_msgpack_user(
        &self,
        user_id: String,
    ) -> Result<HttpMediaTypesApiGetMsgpackUserResponse, xidl_rust_axum::Error> {
        Ok(HttpMediaTypesApiGetMsgpackUserResponse {
            r#return: format!("user:{user_id}"),
            score: 95,
        })
    }
}
