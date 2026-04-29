use crate::generate::http_hir::{HttpMethod as HttpHirMethod, HttpOperation};
use crate::generate::rust_axum::interface::context::DeprecatedContext;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

pub fn http_method_from_hir(method: HttpHirMethod) -> HttpMethod {
    match method {
        HttpHirMethod::Get => HttpMethod::Get,
        HttpHirMethod::Post => HttpMethod::Post,
        HttpHirMethod::Put => HttpMethod::Put,
        HttpHirMethod::Patch => HttpMethod::Patch,
        HttpHirMethod::Delete => HttpMethod::Delete,
        HttpHirMethod::Head => HttpMethod::Head,
        HttpHirMethod::Options => HttpMethod::Options,
    }
}

pub fn http_method_code(method: HttpMethod) -> String {
    match method {
        HttpMethod::Get => "HttpMethod::Get".to_string(),
        HttpMethod::Post => "HttpMethod::Post".to_string(),
        HttpMethod::Put => "HttpMethod::Put".to_string(),
        HttpMethod::Patch => "HttpMethod::Patch".to_string(),
        HttpMethod::Delete => "HttpMethod::Delete".to_string(),
        HttpMethod::Head => "HttpMethod::Head".to_string(),
        HttpMethod::Options => "HttpMethod::Options".to_string(),
    }
}

pub fn http_method_fn(method: HttpMethod) -> String {
    match method {
        HttpMethod::Get => "get".to_string(),
        HttpMethod::Post => "post".to_string(),
        HttpMethod::Put => "put".to_string(),
        HttpMethod::Patch => "patch".to_string(),
        HttpMethod::Delete => "delete".to_string(),
        HttpMethod::Head => "head".to_string(),
        HttpMethod::Options => "options".to_string(),
    }
}

pub fn reqwest_method_code(method: HttpMethod) -> String {
    match method {
        HttpMethod::Get => "GET".to_string(),
        HttpMethod::Post => "POST".to_string(),
        HttpMethod::Put => "PUT".to_string(),
        HttpMethod::Patch => "PATCH".to_string(),
        HttpMethod::Delete => "DELETE".to_string(),
        HttpMethod::Head => "HEAD".to_string(),
        HttpMethod::Options => "OPTIONS".to_string(),
    }
}

pub fn deprecated_context_from_http(http_op: &HttpOperation) -> DeprecatedContext {
    let info = http_op.deprecated.as_ref();
    let deprecated = info.as_ref().map(|value| value.deprecated).unwrap_or(false);
    let since = info.as_ref().and_then(|value| value.since.clone());
    let after = info.as_ref().and_then(|value| value.after.clone());
    let note = info.as_ref().map(|value| {
        let mut note = String::from("Deprecated.");
        if let Some(since) = &value.since {
            note.push_str(&format!(" Since {since}."));
        }
        if let Some(after) = &value.after {
            note.push_str(&format!(" After {after}."));
        }
        note
    });
    DeprecatedContext {
        deprecated,
        since,
        after,
        note,
    }
}
