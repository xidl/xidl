include!(concat!(env!("OUT_DIR"), "/rest_media_types.rs"));

pub struct RestMediaTypesService;

#[async_trait::async_trait]
impl RestMediaTypesApi for RestMediaTypesService {
    async fn submit_profile(
        &self,
        name: String,
        age: u32,
    ) -> Result<RestMediaTypesApiSubmitProfileResponse, xidl_rust_axum::Error> {
        Ok(RestMediaTypesApiSubmitProfileResponse {
            r#return: format!("{name}:{age}"),
            normalized_name: name.to_ascii_uppercase(),
        })
    }

    async fn get_msgpack_user(
        &self,
        user_id: String,
    ) -> Result<RestMediaTypesApiGetMsgpackUserResponse, xidl_rust_axum::Error> {
        Ok(RestMediaTypesApiGetMsgpackUserResponse {
            r#return: format!("user:{user_id}"),
            score: 95,
        })
    }
}
