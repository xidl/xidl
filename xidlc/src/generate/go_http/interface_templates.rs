use serde::Serialize;

#[derive(Serialize)]
pub(super) struct MethodTemplateParam {
    pub(super) field_name: String,
    pub(super) wire_name: String,
}

#[derive(Serialize)]
pub(super) struct ClientBuildRequestTemplate<'a> {
    pub(super) method: ClientBuildRequestMethod<'a>,
}

#[derive(Serialize)]
pub(super) struct ClientBuildRequestMethod<'a> {
    pub(super) struct_prefix: &'a str,
    pub(super) http_method_name: &'a str,
    pub(super) request_body_struct: Option<&'a str>,
    pub(super) request_body_direct_field: Option<&'a str>,
    pub(super) request_body_direct_ty: Option<&'a str>,
    pub(super) request_content_type: &'a str,
    pub(super) response_content_type: &'a str,
    pub(super) body_params: Vec<MethodTemplateParam>,
    pub(super) has_query_params: bool,
    pub(super) has_body_params: bool,
    pub(super) has_security: bool,
    pub(super) query_encode: String,
    pub(super) header_encode: String,
    pub(super) cookie_encode: String,
}

#[derive(Serialize)]
pub(super) struct DecodeResponseTemplate<'a> {
    pub(super) method: DecodeResponseMethod<'a>,
}

#[derive(Serialize)]
pub(super) struct DecodeResponseMethod<'a> {
    pub(super) struct_prefix: &'a str,
    pub(super) response_struct: &'a str,
    pub(super) response_body_struct: Option<&'a str>,
    pub(super) response_body_direct_field: Option<&'a str>,
    pub(super) response_body_direct_ty: Option<&'a str>,
    pub(super) response_content_type: &'a str,
    pub(super) return_ty: Option<&'a str>,
    pub(super) response_body_params: Vec<MethodTemplateParam>,
    pub(super) response_header_decode: String,
    pub(super) response_cookie_decode: String,
}

#[derive(Serialize)]
pub(super) struct RequestBindingTemplate<'a> {
    pub(super) method: RequestBindingMethod<'a>,
}

#[derive(Serialize)]
pub(super) struct RequestBindingMethod<'a> {
    pub(super) is_client_stream: bool,
    pub(super) request_struct: &'a str,
    pub(super) request_body_struct: Option<&'a str>,
    pub(super) request_body_direct_field: Option<&'a str>,
    pub(super) request_body_direct_ty: Option<&'a str>,
    pub(super) request_content_type: &'a str,
    pub(super) body_params: Vec<MethodTemplateParam>,
    pub(super) path_bindings: String,
    pub(super) query_bindings: String,
    pub(super) header_bindings: String,
    pub(super) cookie_bindings: String,
}

#[derive(Serialize)]
pub(super) struct ResponseWriteTemplate<'a> {
    pub(super) method: ResponseWriteMethod<'a>,
    pub(super) value: &'a str,
}

#[derive(Serialize)]
pub(super) struct ResponseWriteMethod<'a> {
    pub(super) response_body_struct: Option<&'a str>,
    pub(super) response_body_direct_field: Option<&'a str>,
    pub(super) response_body_direct_ty: Option<&'a str>,
    pub(super) response_content_type: &'a str,
    pub(super) return_ty: Option<&'a str>,
    pub(super) response_body_params: Vec<MethodTemplateParam>,
    pub(super) response_header_encode: String,
    pub(super) response_cookie_encode: String,
}
