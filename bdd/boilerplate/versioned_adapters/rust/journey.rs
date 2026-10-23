use crate::gen::{v1, v3, version_adapters::*};
use crate::hooks::{Hooks, Storage};
use crate::wire::parcel;
use axum::{extract::State, response::{IntoResponse, Response}, routing::post, Json, Router};
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}}};

pub async fn run() {
    let hooks = Hooks::default();
    let storage = Storage {
        handling_notes: BTreeMap::from([("p1".to_string(), "fragile".to_string())]),
        traces: BTreeMap::from([("p1".to_string(), "trace-1".to_string())]),
    };
    let head = decode_and_upcast_depot_ship(Version::V1, json!({"parcel":parcel()}), &hooks).unwrap().parcel;
    let bad_hooks = Hooks { reject_downcast: true, ..Hooks::default() };
    // The generated ordinary composite response has no Clone implementation.
    let outcome = v3::DepotExchangeResponse { r#return: head.clone(), parcel: head.clone(), etag: "original-etag".into() };
    let return_ptr = outcome.r#return.tracking.as_ptr();
    let inout_ptr = outcome.parcel.tracking.as_ptr();
    let etag_ptr = outcome.etag.as_ptr();
    match downcast_depot_exchange_outcome(Version::V1, Ok(outcome), &storage, &bad_hooks) {
        ProjectionOutcome::Unsupported { head: Ok(original), error } => {
            assert_eq!(original.r#return.tracking.as_ptr(), return_ptr);
            assert_eq!(original.parcel.tracking.as_ptr(), inout_ptr);
            assert_eq!(original.etag.as_ptr(), etag_ptr);
            assert_eq!(error.type_path, "Parcel");
        },
        _ => panic!("a refusal must retain the original non-Clone composite Head"),
    }
    let outcome = v3::DepotExchangeResponse { r#return: head.clone(), parcel: head.clone(), etag: "success-etag".into() };
    match downcast_depot_exchange_outcome(Version::V1, Ok(outcome), &storage, &hooks) {
        ProjectionOutcome::Projected(DepotExchangeOutcome::V1(Ok(value))) => {
            assert_eq!(value.r#return.legacy, "TRACK-1");
            assert_eq!(value.parcel.legacy, "TRACK-1"); assert_eq!(value.etag, "success-etag");
            assert_eq!(value.r#return.handling_note.as_deref(), Some("fragile"), "retired members come back from storage");
        }, _ => panic!("complete response chain must project all outputs"),
    }
    // A retired member reaches only the versions that still declare it; nothing is fabricated.
    assert_eq!(NEEDS.iter().find(|(key, _)| *key == "Parcel").map(|(_, needs)| *needs),
        Some(&[("2026-07", &[][..]), ("2026-04", &["handling_note"][..]), ("2026-01", &["handling_note", "reference"][..])][..]));
    // Operation responses are keyed by their version-map operation.
    assert_eq!(NEEDS.iter().find(|(key, _)| *key == "Depot.trace").map(|(_, needs)| *needs),
        Some(&[("2026-07", &[][..]), ("2026-04", &["trace_id"][..]), ("2026-01", &["trace_id"][..])][..]));
    match downcast_parcel(Version::V2, head.clone(), &storage, &hooks).unwrap() {
        ParcelProjection::V2(value) => assert_eq!(value.handling_note.as_deref(), Some("fragile")),
        _ => panic!("v2 still declares the retired member"),
    }
    let unknown = v3::Parcel { id: "unknown".into(), ..head.clone() };
    match downcast_parcel(Version::V2, unknown, &storage, &hooks).unwrap() {
        ParcelProjection::V2(value) => assert_eq!(value.handling_note, None, "storage without the row omits the member"),
        _ => panic!("v2 still declares the retired member"),
    }
    let cases = [
        v3::DepotGetError::Http404NotFound(v3::NotFound { id: "known".into() }),
        v3::DepotGetError::Http409Busy(v3::Busy { reason: "busy".into() }),
        v3::DepotGetError::Http403Denied(v3::Denied { reason: "denied".into() }),
        v3::DepotGetError::Http429Framework(v3::Framework { reason: "business-framework".into() }),
    ];
    for (error, expected) in cases.into_iter().zip(["known", "busy", "denied", "business-framework"]) {
        match downcast_depot_get_outcome(Version::V1, Err(error), &storage, &hooks) {
            ProjectionOutcome::Projected(DepotGetOutcome::V1(Err(v1::DepotGetError::NotFound(value)))) => assert_eq!(value.id, expected),
            _ => panic!("every typed Head exception must follow the complete downgrade table"),
        }
    }
    assert_eq!(hooks.errors.load(Ordering::SeqCst), 5);
    match downcast_depot_get_outcome(Version::V1, Err(v3::DepotGetError::Framework(xidl_rust_axum::Error::new(503, "runtime"))), &storage, &hooks) {
        ProjectionOutcome::Projected(DepotGetOutcome::V1(Err(v1::DepotGetError::Framework(error)))) => assert_eq!(error.code, 503),
        _ => panic!("runtime Framework must remain distinct from the business exception"),
    }
    let index = v3::StateIndex { states: BTreeMap::from([
        (v3::State::shipping, "in-flight".into()), (v3::State::delivered, "arrived".into()),
    ]) };
    match downcast_depot_get_outcome(Version::V2, Err(v3::DepotGetError::Http418StateIndex(index)), &storage, &hooks) {
        ProjectionOutcome::Unsupported { head: Err(v3::DepotGetError::Http418StateIndex(original)), error } => {
            assert_eq!(original.states.len(), 2);
            assert!(error.reason.contains("not injective"));
        }, _ => panic!("enum-key downgrade collisions must not overwrite a committed result"),
    }
    let reason = "refuse".to_string(); let pointer = reason.as_ptr();
    match downcast_depot_get_outcome(Version::V1, Err(v3::DepotGetError::Http429Framework(v3::Framework { reason })), &storage, &hooks) {
        ProjectionOutcome::Unsupported { head: Err(v3::DepotGetError::Http429Framework(error)), .. } => assert_eq!(error.reason.as_ptr(), pointer),
        _ => panic!("typed-error refusal must retain the original allocation"),
    }
    match downcast_parcel(Version::V3, head.clone(), &storage, &hooks).unwrap() {
        ParcelProjection::V3(value) => assert_eq!(value.audit, "checked"),
        _ => panic!("explicit Head model projection must use the identity branch"),
    }
    // Dispatch hands the handler's stored values to the whole downcast chain.
    let served_head = head.clone();
    let served = dispatch_depot_get(Version::V1, json!({"id":"p1"}), |_, _| async move {
        let stored = Storage { handling_notes: BTreeMap::from([("p1".to_string(), "fragile".to_string())]), ..Storage::default() };
        HeadOutcome { outcome: Ok::<_, v3::DepotGetError>(served_head), stored }
    }, &hooks).await.unwrap();
    match served {
        ProjectionOutcome::Projected(DepotGetOutcome::V1(Ok(value))) => {
            assert_eq!(value.legacy, "TRACK-1"); assert_eq!(value.handling_note.as_deref(), Some("fragile"));
        }, _ => panic!("dispatch must thread stored values through every adjacent step"),
    }
    http_journey().await;
}

#[derive(Default)]
struct Application {
    hooks: Hooks,
    writes: AtomicUsize,
    records: Mutex<BTreeMap<String, (String, v3::State)>>,
}

async fn ship(State(app): State<Arc<Application>>, Json(wire): Json<Value>) -> Response {
    let handler_app = app.clone();
    let result = dispatch_depot_ship(Version::V1, json!({"parcel":wire}), |input, context| async move {
        assert_eq!(context.original_version, Version::V1);
        let fingerprint = serde_json::to_string(&input.parcel).unwrap();
        let mut records = handler_app.records.lock().unwrap();
        if let Some((saved, outcome)) = records.get(&input.parcel.id) {
            if saved != &fingerprint { return Err(v3::DepotShipError::Framework(xidl_rust_axum::Error::new(409, "idempotency conflict"))); }
            return Ok(outcome.clone());
        }
        handler_app.writes.fetch_add(1, Ordering::SeqCst);
        let outcome = if input.parcel.tracking == "special" { v3::State::delivered } else { v3::State::pending };
        records.insert(input.parcel.id, (fingerprint, outcome.clone()));
        Ok(outcome)
    }, &app.hooks).await;
    match result {
        Ok(ProjectionOutcome::Projected(DepotShipOutcome::V1(Ok(value)))) => Json(value).into_response(),
        Ok(ProjectionOutcome::Projected(DepotShipOutcome::V1(Err(error)))) => error.into_response(),
        Ok(ProjectionOutcome::Unsupported { head, error }) => {
            assert!(matches!(head, Ok(v3::State::delivered)));
            // The application renders its own error declared in every frozen contract.
            v1::DepotShipError::CannotRepresent(v1::CannotRepresent { reason: error.reason }).into_response()
        },
        Err(AdaptationError::Validation(reason)) => (axum::http::StatusCode::BAD_REQUEST, reason).into_response(),
        Err(AdaptationError::Unsupported(error)) => v1::DepotShipError::CannotRepresent(v1::CannotRepresent { reason: error.reason }).into_response(),
        _ => panic!("the application resolved V1 before calling dispatch"),
    }
}

async fn http_journey() {
    let app = Arc::new(Application::default());
    let router = Router::new().route("/parcels", post(ship)).with_state(app.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move { axum::serve(listener, router).with_graceful_shutdown(async { let _ = stopped.await; }).await.unwrap(); });
    let client = v1::DepotClient::new(&base);
    let value: v1::Parcel = serde_json::from_value(parcel()).unwrap();
    assert_eq!(client.ship(value.clone()).await.unwrap(), v1::State::pending);
    assert_eq!(client.ship(value.clone()).await.unwrap(), v1::State::pending);
    assert_eq!(app.writes.load(Ordering::SeqCst), 1);
    let mut conflict = value.clone(); conflict.legacy = "different".into();
    assert!(matches!(client.ship(conflict).await, Err(v1::DepotShipError::Framework(error)) if error.code == 409));
    let mut committed = value; committed.id = "committed".into(); committed.legacy = "special".into();
    for _ in 0..2 {
        assert!(matches!(client.ship(committed.clone()).await, Err(v1::DepotShipError::CannotRepresent(_))));
    }
    assert_eq!(app.writes.load(Ordering::SeqCst), 2);
    assert!(matches!(app.records.lock().unwrap().get("committed"), Some((_, v3::State::delivered))));
    let http = xidl_rust_axum::reqwest::Client::new();
    let mut invalid = parcel(); invalid["carrier"] = json!("future");
    let response = http.post(format!("{base}/parcels")).json(&invalid).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 400);
    assert_eq!(app.writes.load(Ordering::SeqCst), 2);
    stop.send(()).unwrap(); server.await.unwrap();
}
