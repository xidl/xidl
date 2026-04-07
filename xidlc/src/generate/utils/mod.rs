pub mod doc;
pub mod filter;
pub mod http;
#[cfg(test)]
mod tests;

pub use doc::doc_lines_from_annotations;
#[cfg(feature = "gen-typescript")]
pub use filter::typescript_format_filter;
#[cfg(any(feature = "gen-c", feature = "gen-cpp"))]
pub use filter::{clang_format_filter, to_case};
pub use filter::{format_timestamp_filter, rust_format_filter};
#[cfg(feature = "gen-openapi")]
pub use http::effective_security;
pub use http::{
    DeprecatedInfo, HttpApiKeyLocation, HttpSecurityOrigin, HttpSecurityProfile,
    HttpSecurityRequirement, HttpStreamCodec, HttpStreamKind, HttpStreamTargetSupport,
    deprecated_info, effective_media_type, effective_security_with_origin, has_optional_annotation,
    http_stream_config, validate_http_annotations, validate_http_stream_method,
    validate_http_stream_target,
};
#[cfg(any(feature = "gen-go-http", feature = "gen-python-http"))]
pub use http::{HttpStreamConfig, annotation_name, annotation_params, normalize_annotation_params};
