use crate::{catalog::Catalog, gen::version_adapters::*, projection::Hooks};
use axum::{
    body::Body,
    extract::State,
    http::{header::SET_COOKIE, HeaderValue, Request, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json, Router,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::Arc;

pub(crate) struct Chain;

#[derive(Deserialize, Serialize)]
struct Filters {
    #[serde(default)]
    media: Vec<String>,
    #[serde(
        default,
        deserialize_with = "xidl_rust_axum::DeserializeFactory::deserialize_present",
        skip_serializing_if = "Option::is_none"
    )]
    remember: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct Preference {
    #[serde(
        default,
        deserialize_with = "xidl_rust_axum::DeserializeFactory::deserialize_present",
        skip_serializing_if = "Option::is_none"
    )]
    media: Option<String>,
}

impl Chain {
    pub(crate) fn router(catalog: Arc<Catalog>) -> Router {
        Router::new()
            .route("/catalog", axum::routing::get(Self::search))
            .route("/catalog/preference", axum::routing::put(Self::remember))
            .with_state(catalog)
    }

    async fn search(
        State(catalog): State<Arc<Catalog>>,
        Extension(version): Extension<Version>,
        request: Request<Body>,
    ) -> xidl_rust_axum::Result<Response> {
        let wire = Self::wire::<Filters>(&request)?;
        let outcome = dispatch_catalog_search(
            version,
            wire,
            |input, _| async move {
                HeadOutcome {
                    outcome: catalog.search(input),
                    stored: Hooks,
                }
            },
            &Hooks,
        )
        .await;
        match Self::projected(outcome)? {
            CatalogSearchOutcome::V1(Ok(value)) => {
                Self::response(Json(value.r#return).into_response(), value.bookmark)
            }
            CatalogSearchOutcome::V2(Ok(value)) => {
                Self::response(Json(value.r#return).into_response(), value.bookmark)
            }
            CatalogSearchOutcome::V1(Err(error)) => Ok(error.into_response()),
            CatalogSearchOutcome::V2(Err(error)) => Ok(error.into_response()),
        }
    }

    async fn remember(
        State(catalog): State<Arc<Catalog>>,
        Extension(version): Extension<Version>,
        request: Request<Body>,
    ) -> xidl_rust_axum::Result<Response> {
        let wire = Self::wire::<Preference>(&request)?;
        let outcome = dispatch_catalog_remember(
            version,
            wire,
            |input, _| async move { catalog.remember(input) },
            &Hooks,
        )
        .await;
        match Self::projected(outcome)? {
            CatalogRememberOutcome::V1(Ok(value)) => {
                Self::response(StatusCode::NO_CONTENT.into_response(), value.bookmark)
            }
            CatalogRememberOutcome::V2(Ok(value)) => {
                Self::response(StatusCode::NO_CONTENT.into_response(), value.bookmark)
            }
            CatalogRememberOutcome::V1(Err(error)) => Ok(error.into_response()),
            CatalogRememberOutcome::V2(Err(error)) => Ok(error.into_response()),
        }
    }

    fn wire<T: DeserializeOwned + Serialize>(
        request: &Request<Body>,
    ) -> xidl_rust_axum::Result<serde_json::Value> {
        let query: T =
            xidl_rust_axum::serde_html_form::from_str(request.uri().query().unwrap_or_default())
                .map_err(|error| xidl_rust_axum::Error::new(400, error.to_string()))?;
        let mut wire = serde_json::to_value(query)
            .map_err(|error| xidl_rust_axum::Error::new(400, error.to_string()))?;
        for header in request.headers().get_all("cookie") {
            let header = header
                .to_str()
                .map_err(|error| xidl_rust_axum::Error::new(400, error.to_string()))?;
            for cookie in xidl_rust_axum::cookie::Cookie::split_parse_encoded(header) {
                let cookie =
                    cookie.map_err(|error| xidl_rust_axum::Error::new(400, error.to_string()))?;
                if cookie.name() == "tenant" && wire.get("tenant").is_none() {
                    wire["tenant"] = serde_json::json!(cookie.value());
                }
            }
        }
        Ok(wire)
    }

    fn projected<T, H>(
        outcome: Result<ProjectionOutcome<T, H>, AdaptationError>,
    ) -> xidl_rust_axum::Result<T> {
        match outcome {
            Ok(ProjectionOutcome::Projected(value)) => Ok(value),
            Ok(ProjectionOutcome::Unsupported { error, .. })
            | Err(AdaptationError::Unsupported(error)) => {
                Err(xidl_rust_axum::Error::new(500, error.reason))
            }
            Err(AdaptationError::Validation(reason)) => {
                Err(xidl_rust_axum::Error::new(400, reason))
            }
        }
    }

    fn response(
        mut response: Response,
        bookmark: Option<String>,
    ) -> xidl_rust_axum::Result<Response> {
        if let Some(value) = bookmark {
            let cookie = xidl_rust_axum::cookie::Cookie::new("bookmark", value)
                .encoded()
                .to_string();
            let header = HeaderValue::from_str(&cookie)
                .map_err(|error| xidl_rust_axum::Error::new(500, error.to_string()))?;
            response.headers_mut().append(SET_COOKIE, header);
        }
        Ok(response)
    }
}
