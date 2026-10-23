use axum::{extract::{Path, Query, State}, response::{IntoResponse, Response}, routing::{get, post}, Json, Router};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[allow(warnings)]
mod gen {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/versioned_adapters.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/versioned_adapters.adapters.rs"));
}
use gen::{v1, v2, version_adapters::*};

struct Hooks;
/// Storage keeps the retired colour while v1 is served; readers resolve it per instance.
struct Storage;
impl StoredValues for Storage {
    fn item_color(&self, source: &v2::Item) -> Option<String> {
        (source.name == "rare").then(|| "teal".to_string())
    }
    /// Never stored: the hook derives the retired reference from the current one.
    fn item_ref_v_1(&self, _: &v2::Item) -> Option<String> { None }
}
impl VersionAdapters for Hooks {
    fn item_upcast_from_v_1(&self, source: &v1::Item) -> VersionedResult<ItemUpcastFromV1Decision> {
        Ok(ItemUpcastFromV1Decision { owner: "ops".into(), note: None, ref_v2: source.ref_v1.clone(), color: IGNORED, ref_v1: IGNORED })
    }
    fn item_downcast_to_v_1(&self, source: &v2::Item, stored: &dyn StoredValues) -> VersionedResult<ItemDowncastToV1Decision> {
        Ok(ItemDowncastToV1Decision { color: stored.item_color(source), ref_v1: source.ref_v2.clone() })
    }
    fn state_downcast_to_v_1(&self, _: &StateDowncastToV1Changes) -> VersionedResult<v1::State> {
        Err(UnsupportedProjection { version: "1.0".into(), direction: Direction::Downcast, type_path: "State".into(), reason: "archived has no v1 representation".into() })
    }
    fn items_create_request_upcast_from_v_1(&self, _: &ItemsCreateRequestV1) -> VersionedResult<ItemsCreateRequestUpcastFromV1Decision> {
        Ok(ItemsCreateRequestUpcastFromV1Decision { actor: "v1-gateway".into() })
    }
    fn items_get_locked_error_downcast_to_v_1(&self, source: &v2::Locked) -> VersionedResult<v1::ItemsGetError> {
        Ok(v1::ItemsGetError::NotFound(v1::NotFound { id: source.owner.clone() }))
    }
}

async fn head_get(input: ItemsGetRequestV2, _: VersionContext) -> HeadOutcome<v2::Item, v2::ItemsGetError, Storage> {
    let outcome = if input.id == "locked" {
        Err(v2::ItemsGetError::Locked(v2::Locked { owner: "ops".into() }))
    } else {
        Ok(v2::Item { ref_v2: format!("ref-{}", input.id), name: input.id, owner: "ops".into(), note: Some("head note".into()) })
    };
    HeadOutcome { outcome, stored: Storage }
}
async fn head_create(input: ItemsCreateRequestV2, _: VersionContext) -> Result<v2::State, xidl_rust_axum::Error> {
    Ok(if input.item.ref_v2 == "special" { v2::State::archived } else { v2::State::draft })
}
async fn read(State(version): State<Version>, Path(id): Path<String>) -> Response {
    match dispatch_items_get(version, json!({"id":id}), head_get, &Hooks).await {
        Ok(ProjectionOutcome::Projected(ItemsGetOutcome::V1(Ok(value)))) => Json(value).into_response(),
        Ok(ProjectionOutcome::Projected(ItemsGetOutcome::V2(Ok(value)))) => Json(value).into_response(),
        Ok(ProjectionOutcome::Projected(ItemsGetOutcome::V1(Err(error)))) => error.into_response(),
        Ok(ProjectionOutcome::Projected(ItemsGetOutcome::V2(Err(error)))) => error.into_response(),
        _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
async fn create(State(version): State<Version>, Query(query): Query<BTreeMap<String,String>>, Json(item): Json<Value>) -> Response {
    let mut wire = json!({"item":item});
    if let Some(actor) = query.get("actor") { wire["actor"] = json!(actor); }
    match dispatch_items_create(version, wire, head_create, &Hooks).await {
        Ok(ProjectionOutcome::Projected(ItemsCreateOutcome::V1(Ok(value)))) => Json(value).into_response(),
        Ok(ProjectionOutcome::Projected(ItemsCreateOutcome::V2(Ok(value)))) => Json(value).into_response(),
        Ok(ProjectionOutcome::Projected(ItemsCreateOutcome::V1(Err(error)))) | Ok(ProjectionOutcome::Projected(ItemsCreateOutcome::V2(Err(error)))) => error.into_response(),
        Err(AdaptationError::Validation(_)) => axum::http::StatusCode::BAD_REQUEST.into_response(),
        _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
#[tokio::main]
async fn main() {
    let contracts = std::env::args().nth(1).expect("Hurl contract directory");
    for (version, name) in [(Version::V1,"v1"), (Version::V2,"v2")] {
        let router = Router::new().route("/items/{id}",get(read)).route("/items",post(create)).with_state(version);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("base_url=http://{}", listener.local_addr().unwrap());
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move { axum::serve(listener,router).with_graceful_shutdown(async { let _ = stopped.await; }).await.unwrap(); });
        let result = tokio::process::Command::new("hurl").args(["--test","--variable",&base,&format!("{contracts}/versioned_adapters.{name}.hurl")]).output().await.unwrap();
        stop.send(()).unwrap(); server.await.unwrap();
        assert!(result.status.success(), "{}{}", String::from_utf8_lossy(&result.stdout), String::from_utf8_lossy(&result.stderr));
    }
    println!("PASS: unchanged Hurl contracts through independent Rust target and one Head");
}
