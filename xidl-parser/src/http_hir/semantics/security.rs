use crate::hir;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::annotations::{
    annotation_name, annotation_params, normalize_annotation_params, parse_string_array,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpApiKeyLocation {
    Header,
    Query,
    Cookie,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpSecurityRequirement {
    HttpBasic,
    HttpBearer,
    ApiKey {
        location: HttpApiKeyLocation,
        name: String,
    },
    OAuth2 {
        scopes: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpSecurityOrigin {
    Interface,
    Method,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpSecurityProfile {
    pub origin: HttpSecurityOrigin,
    pub requirements: Vec<HttpSecurityRequirement>,
}

pub fn effective_security(
    interface_annotations: &[hir::Annotation],
    method_annotations: &[hir::Annotation],
) -> Result<Option<Vec<HttpSecurityRequirement>>, String> {
    let method_security = collect_security(method_annotations)?;
    if method_security.explicit_none {
        return Ok(Some(Vec::new()));
    }
    if !method_security.requirements.is_empty() {
        return Ok(Some(method_security.requirements));
    }
    let interface_security = collect_security(interface_annotations)?;
    if interface_security.explicit_none {
        return Ok(Some(Vec::new()));
    }
    if interface_security.requirements.is_empty() {
        Ok(None)
    } else {
        Ok(Some(interface_security.requirements))
    }
}

pub fn effective_security_with_origin(
    interface_annotations: &[hir::Annotation],
    method_annotations: &[hir::Annotation],
) -> Result<Option<HttpSecurityProfile>, String> {
    let method_security = collect_security(method_annotations)?;
    if method_security.explicit_none {
        return Ok(Some(HttpSecurityProfile {
            origin: HttpSecurityOrigin::Method,
            requirements: Vec::new(),
        }));
    }
    if !method_security.requirements.is_empty() {
        return Ok(Some(HttpSecurityProfile {
            origin: HttpSecurityOrigin::Method,
            requirements: method_security.requirements,
        }));
    }
    let interface_security = collect_security(interface_annotations)?;
    if interface_security.explicit_none {
        return Ok(Some(HttpSecurityProfile {
            origin: HttpSecurityOrigin::Interface,
            requirements: Vec::new(),
        }));
    }
    if interface_security.requirements.is_empty() {
        Ok(None)
    } else {
        Ok(Some(HttpSecurityProfile {
            origin: HttpSecurityOrigin::Interface,
            requirements: interface_security.requirements,
        }))
    }
}

pub(crate) struct SecurityCollection {
    pub(crate) explicit_none: bool,
    pub(crate) requirements: Vec<HttpSecurityRequirement>,
}

pub(crate) fn collect_security(
    annotations: &[hir::Annotation],
) -> Result<SecurityCollection, String> {
    let mut explicit_none = false;
    let mut requirements = Vec::new();
    let mut singleton_names = BTreeSet::new();
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if name.eq_ignore_ascii_case("no_security") {
            explicit_none = true;
            continue;
        }
        let requirement = if name.eq_ignore_ascii_case("http_basic") {
            ensure_singleton(&mut singleton_names, "http_basic")?;
            Some(HttpSecurityRequirement::HttpBasic)
        } else if name.eq_ignore_ascii_case("http_bearer") {
            ensure_singleton(&mut singleton_names, "http_bearer")?;
            Some(HttpSecurityRequirement::HttpBearer)
        } else if name.eq_ignore_ascii_case("api_key") {
            Some(parse_api_key(annotation)?)
        } else if name.eq_ignore_ascii_case("oauth2") {
            Some(parse_oauth2(annotation))
        } else {
            None
        };
        if let Some(requirement) = requirement {
            requirements.push(requirement);
        }
    }
    if explicit_none && !requirements.is_empty() {
        return Err("@no_security cannot be combined with other security annotations".to_string());
    }
    Ok(SecurityCollection {
        explicit_none,
        requirements,
    })
}

fn ensure_singleton(names: &mut BTreeSet<&'static str>, value: &'static str) -> Result<(), String> {
    if !names.insert(value) {
        return Err(format!("duplicate @{value} annotation"));
    }
    Ok(())
}

fn parse_api_key(annotation: &hir::Annotation) -> Result<HttpSecurityRequirement, String> {
    let params = annotation_params(annotation)
        .ok_or_else(|| "@api_key requires in=... and name=...".to_string())?;
    let params = normalize_annotation_params(params);
    let location = match params.get("in").map(|value| value.to_ascii_lowercase()) {
        Some(value) if value == "header" => HttpApiKeyLocation::Header,
        Some(value) if value == "query" => HttpApiKeyLocation::Query,
        Some(value) if value == "cookie" => HttpApiKeyLocation::Cookie,
        Some(value) => {
            return Err(format!(
                "@api_key(in=...) must be one of header|query|cookie, got '{value}'"
            ));
        }
        None => return Err("@api_key requires non-empty in=...".to_string()),
    };
    let name = params
        .get("name")
        .cloned()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "@api_key requires non-empty name=...".to_string())?;
    Ok(HttpSecurityRequirement::ApiKey { location, name })
}

fn parse_oauth2(annotation: &hir::Annotation) -> HttpSecurityRequirement {
    let params = annotation_params(annotation)
        .map(normalize_annotation_params)
        .unwrap_or_default();
    let scopes = params
        .get("scopes")
        .map(|value| parse_string_array(value))
        .unwrap_or_default();
    HttpSecurityRequirement::OAuth2 { scopes }
}
