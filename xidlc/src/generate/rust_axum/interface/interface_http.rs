use crate::generate::rust_axum::interface::{ApiKeyContext, DeprecatedContext, HttpMethod};
use xidl_parser::hir;
use xidl_parser::rest_hir::{
    HttpMethod as RestHirMethod, HttpOperation,
    semantics::{
        HttpApiKeyLocation, HttpSecurityOrigin, HttpSecurityProfile, HttpSecurityRequirement,
    },
};

pub(crate) fn http_method_from_hir(method: RestHirMethod) -> HttpMethod {
    match method {
        RestHirMethod::Get => HttpMethod::Get,
        RestHirMethod::Post => HttpMethod::Post,
        RestHirMethod::Put => HttpMethod::Put,
        RestHirMethod::Patch => HttpMethod::Patch,
        RestHirMethod::Delete => HttpMethod::Delete,
        RestHirMethod::Head => HttpMethod::Head,
        RestHirMethod::Options => HttpMethod::Options,
    }
}

pub(crate) fn deprecated_context_from_http(http_op: &HttpOperation) -> DeprecatedContext {
    let info = http_op.meta.deprecated.as_ref();
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

pub(crate) fn attr_operation_names(attr: &hir::AttrDcl) -> Vec<String> {
    match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => match &spec.declarator {
            hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => vec![
                format!("get_attribute_{}", decl.0),
                format!("watch_attribute_{}", decl.0),
            ],
            hir::ReadonlyAttrDeclarator::RaisesExpr(_) => Vec::new(),
        },
        hir::AttrDclInner::AttrSpec(spec) => match &spec.declarator {
            hir::AttrDeclarator::SimpleDeclarator(list) => list
                .iter()
                .flat_map(|decl| {
                    [
                        format!("get_attribute_{}", decl.0),
                        format!("set_attribute_{}", decl.0),
                        format!("watch_attribute_{}", decl.0),
                    ]
                })
                .collect(),
            hir::AttrDeclarator::WithRaises { declarator, .. } => vec![
                format!("get_attribute_{}", declarator.0),
                format!("set_attribute_{}", declarator.0),
                format!("watch_attribute_{}", declarator.0),
            ],
        },
    }
}

pub(crate) struct SecurityContext {
    pub(crate) has_basic_auth: bool,
    pub(crate) has_bearer_auth: bool,
    pub(crate) api_key_requirements: Vec<ApiKeyContext>,
    pub(crate) auth_source_interface: bool,
    pub(crate) auth_source_method: bool,
    pub(crate) auth_ty: String,
}

pub(crate) fn security_context(profile: &Option<HttpSecurityProfile>) -> SecurityContext {
    let (security, auth_source_interface, auth_source_method) = match profile {
        None => (None, false, false),
        Some(HttpSecurityProfile {
            origin,
            requirements,
        }) => (
            Some(requirements.clone()),
            matches!(origin, HttpSecurityOrigin::Interface),
            matches!(origin, HttpSecurityOrigin::Method),
        ),
    };
    let has_basic_auth = security
        .as_ref()
        .map(|reqs| {
            reqs.iter()
                .any(|req| matches!(req, HttpSecurityRequirement::HttpBasic))
        })
        .unwrap_or(false);
    let has_bearer_auth = security
        .as_ref()
        .map(|reqs| {
            reqs.iter()
                .any(|req| matches!(req, HttpSecurityRequirement::HttpBearer))
        })
        .unwrap_or(false);
    let api_key_requirements = security
        .as_ref()
        .map(|reqs| {
            reqs.iter()
                .filter_map(|req| match req {
                    HttpSecurityRequirement::ApiKey { location, name } => Some(ApiKeyContext {
                        location: api_key_location(location).to_string(),
                        name: name.clone(),
                    }),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let auth_ty = if has_basic_auth {
        "xidl_rust_axum::auth::basic::BasicAuth".to_string()
    } else if has_bearer_auth {
        "xidl_rust_axum::auth::bearer::BearerAuth".to_string()
    } else {
        String::new()
    };

    SecurityContext {
        has_basic_auth,
        has_bearer_auth,
        api_key_requirements,
        auth_source_interface,
        auth_source_method,
        auth_ty,
    }
}

fn api_key_location(location: &HttpApiKeyLocation) -> &'static str {
    match location {
        HttpApiKeyLocation::Header => "Header",
        HttpApiKeyLocation::Query => "Query",
        HttpApiKeyLocation::Cookie => "Cookie",
    }
}

pub(crate) fn http_method_code(method: HttpMethod) -> String {
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

pub(crate) fn http_method_fn(method: HttpMethod) -> String {
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

pub(crate) fn reqwest_method_code(method: HttpMethod) -> String {
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
