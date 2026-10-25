use crate::{catalog::Catalog, gen::version_adapters::*, projection::Hooks};
use axum::{
    body::Body,
    extract::State,
    http::Request,
    response::{IntoResponse, Response},
    Extension, Json, Router,
};
use serde_json::json;
use std::sync::Arc;

pub(crate) struct Chain;

#[derive(serde::Deserialize)]
struct Filters {
    #[serde(default)]
    media: Vec<String>,
}

impl Chain {
    pub(crate) fn router(catalog: Arc<Catalog>) -> Router {
        Router::new()
            .route("/catalog", axum::routing::get(Self::search))
            .with_state(catalog)
    }

    async fn search(
        State(catalog): State<Arc<Catalog>>,
        Extension(version): Extension<Version>,
        request: Request<Body>,
    ) -> Response {
        match Self::execute(catalog, version, request).await {
            Ok(response) => response,
            Err(error) => error.into_response(),
        }
    }

    async fn execute(
        catalog: Arc<Catalog>,
        version: Version,
        request: Request<Body>,
    ) -> xidl_rust_axum::Result<Response> {
        let filters: Filters =
            xidl_rust_axum::serde_html_form::from_str(request.uri().query().unwrap_or_default())
                .map_err(|error| xidl_rust_axum::Error::new(400, error.to_string()))?;
        let mut wire = json!({ "media": filters.media });
        for header in request.headers().get_all("cookie") {
            let header = header
                .to_str()
                .map_err(|error| xidl_rust_axum::Error::new(400, error.to_string()))?;
            for cookie in xidl_rust_axum::cookie::Cookie::split_parse_encoded(header) {
                let cookie =
                    cookie.map_err(|error| xidl_rust_axum::Error::new(400, error.to_string()))?;
                if cookie.name() == "tenant" && wire.get("tenant").is_none() {
                    wire["tenant"] = json!(cookie.value());
                }
            }
        }
        Ok(
            match dispatch_catalog_search(
                version,
                wire,
                |input, _| async move { catalog.search(input) },
                &Hooks,
            )
            .await
            {
                Ok(ProjectionOutcome::Projected(CatalogSearchOutcome::V1(Ok(value)))) => {
                    Json(value).into_response()
                }
                Ok(ProjectionOutcome::Projected(CatalogSearchOutcome::V2(Ok(value)))) => {
                    Json(value).into_response()
                }
                Ok(ProjectionOutcome::Projected(CatalogSearchOutcome::V1(Err(error)))) => {
                    error.into_response()
                }
                Ok(ProjectionOutcome::Projected(CatalogSearchOutcome::V2(Err(error)))) => {
                    error.into_response()
                }
                Ok(ProjectionOutcome::Unsupported { error, .. })
                | Err(AdaptationError::Unsupported(error)) => {
                    xidl_rust_axum::Error::new(500, error.reason).into_response()
                }
                Err(AdaptationError::Validation(reason)) => {
                    xidl_rust_axum::Error::new(400, reason).into_response()
                }
            },
        )
    }
}
