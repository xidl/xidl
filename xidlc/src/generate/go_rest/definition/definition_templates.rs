use serde::Serialize;

#[derive(Serialize)]
pub(super) struct RequestBindTemplate<'a> {
    pub(super) field: &'a str,
    pub(super) call: &'a str,
    pub(super) ty: &'a str,
    pub(super) optional: bool,
    pub(super) wire_name: &'a str,
    pub(super) source_kind: &'a str,
}

#[derive(Serialize)]
pub(super) struct EncodeTemplate<'a> {
    pub(super) wire_name: &'a str,
    pub(super) field: &'a str,
    pub(super) ty: &'a str,
    pub(super) optional: bool,
}

#[derive(Serialize)]
pub(super) struct ResponseEncodeTemplate<'a> {
    pub(super) wire_name: &'a str,
    pub(super) field: &'a str,
    pub(super) ty: &'a str,
}

#[derive(Serialize)]
pub(super) struct ResponseHeaderDecodeTemplate<'a> {
    pub(super) wire_name: &'a str,
    pub(super) field: &'a str,
    pub(super) ty: &'a str,
}

#[derive(Serialize)]
pub(super) struct ResponseCookieDecodeTemplate<'a> {
    pub(super) wire_name: &'a str,
    pub(super) field_name: &'a str,
    pub(super) ty: &'a str,
}

#[derive(Serialize)]
pub(super) struct FormatPathTemplate<'a> {
    pub(super) struct_prefix: &'a str,
    pub(super) request_struct: &'a str,
    pub(super) raw_path: &'a str,
    pub(super) trim_query_template: bool,
    pub(super) replacements: Vec<FormatPathReplacement>,
}

#[derive(Serialize)]
pub(super) struct FormatPathReplacement {
    pub(super) replacement: String,
    pub(super) replacement_catchall: String,
    pub(super) expr: String,
}
