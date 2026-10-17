use crate::openapi::security::{
    ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityRequirement, SecurityScheme,
};
use std::collections::BTreeMap;
use xidl_parser::hir::HttpParameterLocation;
use xidl_parser::rest_hir::semantics::HttpSecurityRequirement;

pub(crate) fn openapi_security_requirement(
    requirement: HttpSecurityRequirement,
) -> SecurityRequirement {
    match requirement {
        HttpSecurityRequirement::HttpBasic => {
            SecurityRequirement::new("http_basic", Vec::<String>::new())
        }
        HttpSecurityRequirement::HttpBearer => {
            SecurityRequirement::new("http_bearer", Vec::<String>::new())
        }
        HttpSecurityRequirement::ApiKey { location, name } => {
            SecurityRequirement::new(api_key_scheme_name(&location, &name), Vec::<String>::new())
        }
    }
}

pub(crate) fn register_security_schemes(
    store: &mut BTreeMap<String, SecurityScheme>,
    security: &[HttpSecurityRequirement],
) {
    for requirement in security {
        match requirement {
            HttpSecurityRequirement::HttpBasic => {
                store
                    .entry("http_basic".to_string())
                    .or_insert_with(|| SecurityScheme::Http(Http::new(HttpAuthScheme::Basic)));
            }
            HttpSecurityRequirement::HttpBearer => {
                store
                    .entry("http_bearer".to_string())
                    .or_insert_with(|| SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)));
            }
            HttpSecurityRequirement::ApiKey { location, name } => {
                let key = api_key_scheme_name(location, name);
                store.entry(key).or_insert_with(|| match location {
                    HttpParameterLocation::Header => {
                        SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new(name.clone())))
                    }
                    HttpParameterLocation::Query => {
                        SecurityScheme::ApiKey(ApiKey::Query(ApiKeyValue::new(name.clone())))
                    }
                    HttpParameterLocation::Cookie => {
                        SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(name.clone())))
                    }
                });
            }
        }
    }
}

fn api_key_scheme_name(location: &HttpParameterLocation, name: &str) -> String {
    let location = match location {
        HttpParameterLocation::Header => "header",
        HttpParameterLocation::Query => "query",
        HttpParameterLocation::Cookie => "cookie",
    };
    format!("api_key-{location}-{}", name.to_ascii_lowercase())
}
