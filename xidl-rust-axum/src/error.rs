use std::borrow::Cow;

use axum::response::{IntoResponse, Response};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct Error {
    pub code: u16,
    pub message: Cow<'static, str>,
}

macro_rules! status_codes {
    ($(
        $(#[$docs:meta])*
        ($code:literal, $name:ident, $msg: literal);)*) => {
        $(
            $(#[$docs])*
            pub const fn $name() -> Self {
                Self {
                    code: $code,
                    message: Cow::Borrowed($msg)
                }
            }
        )*
    };
}

impl Error {
    pub fn new(code: u16, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn message(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(500, message)
    }

    // copy from http

    status_codes! {
        /// 100 Continue
        /// [[RFC9110, Section 15.2.1](https://datatracker.ietf.org/doc/html/rfc9110#section-15.2.1)]
        (100, r#continue, "Continue");
        /// 101 Switching Protocols
        /// [[RFC9110, Section 15.2.2](https://datatracker.ietf.org/doc/html/rfc9110#section-15.2.2)]
        (101, switching_protocols, "Switching Protocols");
        /// 102 Processing
        /// [[RFC2518, Section 10.1](https://datatracker.ietf.org/doc/html/rfc2518#section-10.1)]
        (102, processing, "Processing");
        /// 103 Early Hints
        /// [[RFC8297, Section 2](https://datatracker.ietf.org/doc/html/rfc8297#section-2)]
        (103, early_hints, "Early Hints");

        /// 200 OK
        /// [[RFC9110, Section 15.3.1](https://datatracker.ietf.org/doc/html/rfc9110#section-15.3.1)]
        (200, ok, "OK");
        /// 201 Created
        /// [[RFC9110, Section 15.3.2](https://datatracker.ietf.org/doc/html/rfc9110#section-15.3.2)]
        (201, created, "Created");
        /// 202 Accepted
        /// [[RFC9110, Section 15.3.3](https://datatracker.ietf.org/doc/html/rfc9110#section-15.3.3)]
        (202, accepted, "Accepted");
        /// 203 Non-Authoritative Information
        /// [[RFC9110, Section 15.3.4](https://datatracker.ietf.org/doc/html/rfc9110#section-15.3.4)]
        (203, non_authoritative_information, "Non Authoritative Information");
        /// 204 No Content
        /// [[RFC9110, Section 15.3.5](https://datatracker.ietf.org/doc/html/rfc9110#section-15.3.5)]
        (204, no_content, "No Content");
        /// 205 Reset Content
        /// [[RFC9110, Section 15.3.6](https://datatracker.ietf.org/doc/html/rfc9110#section-15.3.6)]
        (205, reset_content, "Reset Content");
        /// 206 Partial Content
        /// [[RFC9110, Section 15.3.7](https://datatracker.ietf.org/doc/html/rfc9110#section-15.3.7)]
        (206, partial_content, "Partial Content");
        /// 207 Multi-Status
        /// [[RFC4918, Section 11.1](https://datatracker.ietf.org/doc/html/rfc4918#section-11.1)]
        (207, multi_status, "Multi-Status");
        /// 208 Already Reported
        /// [[RFC5842, Section 7.1](https://datatracker.ietf.org/doc/html/rfc5842#section-7.1)]
        (208, already_reported, "Already Reported");

        /// 226 IM Used
        /// [[RFC3229, Section 10.4.1](https://datatracker.ietf.org/doc/html/rfc3229#section-10.4.1)]
        (226, im_used, "IM Used");

        /// 300 Multiple Choices
        /// [[RFC9110, Section 15.4.1](https://datatracker.ietf.org/doc/html/rfc9110#section-15.4.1)]
        (300, multiple_choices, "Multiple Choices");
        /// 301 Moved Permanently
        /// [[RFC9110, Section 15.4.2](https://datatracker.ietf.org/doc/html/rfc9110#section-15.4.2)]
        (301, moved_permanently, "Moved Permanently");
        /// 302 Found
        /// [[RFC9110, Section 15.4.3](https://datatracker.ietf.org/doc/html/rfc9110#section-15.4.3)]
        (302, found, "Found");
        /// 303 See Other
        /// [[RFC9110, Section 15.4.4](https://datatracker.ietf.org/doc/html/rfc9110#section-15.4.4)]
        (303, see_other, "See Other");
        /// 304 Not Modified
        /// [[RFC9110, Section 15.4.5](https://datatracker.ietf.org/doc/html/rfc9110#section-15.4.5)]
        (304, not_modified, "Not Modified");
        /// 305 Use Proxy
        /// [[RFC9110, Section 15.4.6](https://datatracker.ietf.org/doc/html/rfc9110#section-15.4.6)]
        (305, use_proxy, "Use Proxy");
        /// 307 Temporary Redirect
        /// [[RFC9110, Section 15.4.7](https://datatracker.ietf.org/doc/html/rfc9110#section-15.4.7)]
        (307, temporary_redirect, "Temporary Redirect");
        /// 308 Permanent Redirect
        /// [[RFC9110, Section 15.4.8](https://datatracker.ietf.org/doc/html/rfc9110#section-15.4.8)]
        (308, permanent_redirect, "Permanent Redirect");

        /// 400 Bad Request
        /// [[RFC9110, Section 15.5.1](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.1)]
        (400, bad_request, "Bad Request");
        /// 401 Unauthorized
        /// [[RFC9110, Section 15.5.2](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.2)]
        (401, unauthorized, "Unauthorized");
        /// 402 Payment Required
        /// [[RFC9110, Section 15.5.3](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.3)]
        (402, payment_required, "Payment Required");
        /// 403 Forbidden
        /// [[RFC9110, Section 15.5.4](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.4)]
        (403, forbidden, "Forbidden");
        /// 404 Not Found
        /// [[RFC9110, Section 15.5.5](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.5)]
        (404, not_found, "Not Found");
        /// 405 Method Not Allowed
        /// [[RFC9110, Section 15.5.6](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.6)]
        (405, method_not_allowed, "Method Not Allowed");
        /// 406 Not Acceptable
        /// [[RFC9110, Section 15.5.7](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.7)]
        (406, not_acceptable, "Not Acceptable");
        /// 407 Proxy Authentication Required
        /// [[RFC9110, Section 15.5.8](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.8)]
        (407, proxy_authentication_required, "Proxy Authentication Required");
        /// 408 Request Timeout
        /// [[RFC9110, Section 15.5.9](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.9)]
        (408, request_timeout, "Request Timeout");
        /// 409 Conflict
        /// [[RFC9110, Section 15.5.10](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.10)]
        (409, conflict, "Conflict");
        /// 410 Gone
        /// [[RFC9110, Section 15.5.11](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.11)]
        (410, gone, "Gone");
        /// 411 Length Required
        /// [[RFC9110, Section 15.5.12](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.12)]
        (411, length_required, "Length Required");
        /// 412 Precondition Failed
        /// [[RFC9110, Section 15.5.13](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.13)]
        (412, precondition_failed, "Precondition Failed");
        /// 413 Payload Too Large
        /// [[RFC9110, Section 15.5.14](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.14)]
        (413, payload_too_large, "Payload Too Large");
        /// 414 URI Too Long
        /// [[RFC9110, Section 15.5.15](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.15)]
        (414, uri_too_long, "URI Too Long");
        /// 415 Unsupported Media Type
        /// [[RFC9110, Section 15.5.16](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.16)]
        (415, unsupported_media_type, "Unsupported Media Type");
        /// 416 Range Not Satisfiable
        /// [[RFC9110, Section 15.5.17](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.17)]
        (416, range_not_satisfiable, "Range Not Satisfiable");
        /// 417 Expectation Failed
        /// [[RFC9110, Section 15.5.18](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.18)]
        (417, expectation_failed, "Expectation Failed");
        /// 418 I'm a teapot
        /// [curiously not registered by IANA but [RFC2324, Section 2.3.2](https://datatracker.ietf.org/doc/html/rfc2324#section-2.3.2)]
        (418, im_a_teapot, "I'm a teapot");

        /// 421 Misdirected Request
        /// [[RFC9110, Section 15.5.20](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.20)]
        (421, misdirected_request, "Misdirected Request");
        /// 422 Unprocessable Entity
        /// [[RFC9110, Section 15.5.21](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.21)]
        (422, unprocessable_entity, "Unprocessable Entity");
        /// 423 Locked
        /// [[RFC4918, Section 11.3](https://datatracker.ietf.org/doc/html/rfc4918#section-11.3)]
        (423, locked, "Locked");
        /// 424 Failed Dependency
        /// [[RFC4918, Section 11.4](https://tools.ietf.org/html/rfc4918#section-11.4)]
        (424, failed_dependency, "Failed Dependency");

        /// 425 Too early
        /// [[RFC8470, Section 5.2](https://httpwg.org/specs/rfc8470.html#status)]
        (425, too_early, "Too Early");

        /// 426 Upgrade Required
        /// [[RFC9110, Section 15.5.22](https://datatracker.ietf.org/doc/html/rfc9110#section-15.5.22)]
        (426, upgrade_required, "Upgrade Required");

        /// 428 Precondition Required
        /// [[RFC6585, Section 3](https://datatracker.ietf.org/doc/html/rfc6585#section-3)]
        (428, precondition_required, "Precondition Required");
        /// 429 Too Many Requests
        /// [[RFC6585, Section 4](https://datatracker.ietf.org/doc/html/rfc6585#section-4)]
        (429, too_many_requests, "Too Many Requests");

        /// 431 Request Header Fields Too Large
        /// [[RFC6585, Section 5](https://datatracker.ietf.org/doc/html/rfc6585#section-5)]
        (431, request_header_fields_too_large, "Request Header Fields Too Large");

        /// 451 Unavailable For Legal Reasons
        /// [[RFC7725, Section 3](https://tools.ietf.org/html/rfc7725#section-3)]
        (451, unavailable_for_legal_reasons, "Unavailable For Legal Reasons");

        /// 500 Internal Server Error
        /// [[RFC9110, Section 15.6.1](https://datatracker.ietf.org/doc/html/rfc9110#section-15.6.1)]
        (500, internal_server_error, "Internal Server Error");
        /// 501 Not Implemented
        /// [[RFC9110, Section 15.6.2](https://datatracker.ietf.org/doc/html/rfc9110#section-15.6.2)]
        (501, not_implemented, "Not Implemented");
        /// 502 Bad Gateway
        /// [[RFC9110, Section 15.6.3](https://datatracker.ietf.org/doc/html/rfc9110#section-15.6.3)]
        (502, bad_gateway, "Bad Gateway");
        /// 503 Service Unavailable
        /// [[RFC9110, Section 15.6.4](https://datatracker.ietf.org/doc/html/rfc9110#section-15.6.4)]
        (503, service_unavailable, "Service Unavailable");
        /// 504 Gateway Timeout
        /// [[RFC9110, Section 15.6.5](https://datatracker.ietf.org/doc/html/rfc9110#section-15.6.5)]
        (504, gateway_timeout, "Gateway Timeout");
        /// 505 HTTP Version Not Supported
        /// [[RFC9110, Section 15.6.6](https://datatracker.ietf.org/doc/html/rfc9110#section-15.6.6)]
        (505, http_version_not_supported, "HTTP Version Not Supported");
        /// 506 Variant Also Negotiates
        /// [[RFC2295, Section 8.1](https://datatracker.ietf.org/doc/html/rfc2295#section-8.1)]
        (506, variant_also_negotiates, "Variant Also Negotiates");
        /// 507 Insufficient Storage
        /// [[RFC4918, Section 11.5](https://datatracker.ietf.org/doc/html/rfc4918#section-11.5)]
        (507, insufficient_storage, "Insufficient Storage");
        /// 508 Loop Detected
        /// [[RFC5842, Section 7.2](https://datatracker.ietf.org/doc/html/rfc5842#section-7.2)]
        (508, loop_detected, "Loop Detected");

        /// 510 Not Extended
        /// [[RFC2774, Section 7](https://datatracker.ietf.org/doc/html/rfc2774#section-7)]
        (510, not_extended, "Not Extended");
        /// 511 Network Authentication Required
        /// [[RFC6585, Section 6](https://datatracker.ietf.org/doc/html/rfc6585#section-6)]
        (511, network_authentication_required, "Network Authentication Required");
    }
}

impl From<Error> for ErrorBody {
    fn from(err: Error) -> Self {
        Self {
            code: err.code,
            msg: err.message,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let body: ErrorBody = self.into();
        (StatusCode::from_u16(body.code).unwrap(), axum::Json(body)).into_response()
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: u16,
    pub msg: Cow<'static, str>,
}
