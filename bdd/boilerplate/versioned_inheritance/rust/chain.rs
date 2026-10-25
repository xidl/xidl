use crate::{
    gen::{v3, version_adapters::*},
    hooks::Hooks,
    store::Store,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;

pub fn selected(headers: &HeaderMap) -> Option<Version> {
    match headers
        .get("x-address-version")
        .and_then(|value| value.to_str().ok())
    {
        None | Some("2026-07-01") => Some(Version::V3),
        Some("2026-01-01") => Some(Version::V1),
        Some("2026-04-01") => Some(Version::V2),
        _ => None,
    }
}

fn refusal(error: AdaptationError) -> Response {
    match error {
        AdaptationError::Validation(reason) => {
            xidl_rust_axum::Error::new(400, reason).into_response()
        }
        AdaptationError::Unsupported(error) => projection(error),
    }
}

fn projection(error: UnsupportedProjection) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({"type_path":error.type_path,"reason":error.reason})),
    )
        .into_response()
}

pub async fn read(
    State(store): State<Arc<Store>>,
    Path((tenant, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let version = selected(&headers).unwrap();
    let result = dispatch_shipping_addresses_read(
        version,
        json!({"tenant":tenant,"id":id}),
        |input, _| async move {
            HeadOutcome {
                outcome: store.current(&input.tenant, &input.id).ok_or_else(|| {
                    v3::shipping::AddressesReadError::Missing(v3::shipping::Missing {
                        reason: "address missing".into(),
                    })
                }),
                stored: store.readers(&input.tenant),
            }
        },
        &Hooks,
    )
    .await;
    match result {
        Err(error) => refusal(error),
        Ok(ProjectionOutcome::Unsupported { error, .. }) => projection(error),
        Ok(ProjectionOutcome::Projected(outcome)) => match outcome {
            ShippingAddressesReadOutcome::V1(value) => value.map(Json).into_response(),
            ShippingAddressesReadOutcome::V2(value) => value.map(Json).into_response(),
            ShippingAddressesReadOutcome::V3(value) => value.map(Json).into_response(),
        },
    }
}

pub async fn update(
    State(store): State<Arc<Store>>,
    Path((tenant, id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(patch): Json<Value>,
) -> Response {
    let Some(current) = store.current(&tenant, &id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"reason":"address missing"})),
        )
            .into_response();
    };
    let result = dispatch_shipping_addresses_update(
        selected(&headers).unwrap(),
        json!({"tenant":tenant,"id":id,"patch":patch}),
        current,
        |input, _, bag| async move { store.update(input, bag) },
        &Hooks,
    )
    .await;
    match result {
        Err(error) => refusal(error),
        Ok(ProjectionOutcome::Unsupported { error, .. }) => projection(error),
        Ok(ProjectionOutcome::Projected(outcome)) => match outcome {
            ShippingAddressesUpdateOutcome::V1(value) => value.map(Json).into_response(),
            ShippingAddressesUpdateOutcome::V2(value) => value.map(Json).into_response(),
            ShippingAddressesUpdateOutcome::V3(value) => value.map(Json).into_response(),
        },
    }
}
