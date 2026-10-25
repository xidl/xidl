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
        ..Storage::default()
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
    // A member added inside the element type leaves every older view with the
    // element count and keys intact: the sequence, the map and the fixed
    // array all drop the newest member per element instead of failing.
    let zoned = v3::Parcel {
        address: v3::Address { zone: Some("dock-3".into()), ..head.address.clone() },
        stops: head.stops.iter().cloned()
            .map(|stop| v3::Address { zone: Some("gate-2".into()), ..stop }).collect(),
        by_tag: head.by_tag.iter()
            .map(|(tag, stop)| (tag.clone(), v3::Address { zone: Some("rear".into()), ..stop.clone() }))
            .collect(),
        ..head.clone()
    };
    for version in [Version::V1, Version::V2] {
        let wire = match downcast_parcel(version, zoned.clone(), &storage, &hooks).unwrap() {
            ParcelProjection::V1(value) => serde_json::to_value(serde_json::json!({
                "zip": value.address.zip, "stops": value.stops, "by_tag": value.by_tag,
            })).unwrap(),
            ParcelProjection::V2(value) => serde_json::to_value(serde_json::json!({
                "zip": value.address.zip, "stops": value.stops, "by_tag": value.by_tag,
            })).unwrap(),
            _ => panic!("the zoned parcel must project"),
        };
        assert_eq!(wire["zip"], "10001");
        assert_eq!(wire["stops"].as_array().map(Vec::len), Some(1));
        assert_eq!(wire["by_tag"].as_object().map(|map| map.len()), Some(1));
        assert!(!wire.to_string().contains("zone"), "{version:?}: no element leaks the newest member");
    }
    // A member retired inside every container element returns per element
    // from storage, an element without its row omits the member, and a
    // business refusal surfaces the full container path.
    let element = |zip: &str| v3::Address { zip: zip.into(), ..head.address.clone() };
    let gated_storage = Storage {
        gates: BTreeMap::from([("20002".to_string(), "B-12".to_string())]),
        ..storage.clone()
    };
    let gated = v3::Parcel {
        stops: vec![element("20002")],
        by_tag: BTreeMap::from([("dock".to_string(), element("30003"))]),
        ..head.clone()
    };
    for version in [Version::V1, Version::V2] {
        let (stop_gate, dock_gate) = match downcast_parcel(version, gated.clone(), &gated_storage, &hooks).unwrap() {
            ParcelProjection::V1(value) => (value.stops[0].gate.clone(), value.by_tag["dock"].gate.clone()),
            ParcelProjection::V2(value) => (value.stops[0].gate.clone(), value.by_tag["dock"].gate.clone()),
            _ => panic!("the gated parcel must project"),
        };
        assert_eq!(stop_gate.as_deref(), Some("B-12"));
        assert_eq!(dock_gate, None, "storage without the element row omits the member");
    }
    let refused = v3::Parcel { stops: vec![element("20002"), element("90001")], ..gated.clone() };
    match downcast_parcel(Version::V2, refused, &gated_storage, &hooks) {
        Err(error) => assert_eq!(error.type_path, "Parcel.stops.Address.gate",
            "the refusal names the sequence element's member"),
        Ok(_) => panic!("a gated dock must refuse the whole projection"),
    }
    let refused = v3::Parcel {
        by_tag: BTreeMap::from([("dock".to_string(), element("90001"))]),
        ..gated.clone()
    };
    match downcast_parcel(Version::V2, refused, &gated_storage, &hooks) {
        Err(error) => assert_eq!(error.type_path, "Parcel.by_tag.Address.gate",
            "the refusal names the map element's member"),
        Ok(_) => panic!("a gated dock must refuse the whole projection"),
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
    // The update round trip: a patch retiring a request member on Head must
    // still let older callers persist it, and writes must read back unchanged.
    let update_notes = std::sync::Mutex::new(BTreeMap::from([
        ("p1".to_string(), "fragile".to_string()),
    ]));
    let update_writes = AtomicUsize::new(0);
    let resource = std::sync::Mutex::new(head.clone());
    async fn refused_handler(
        _: DepotUpdateRequestV3, _: VersionContext, _: DepotUpdateStored,
    ) -> HeadOutcome<v3::Parcel, v3::DepotUpdateError, Storage> {
        unreachable!("validation refusals must never reach the handler")
    }
    match dispatch_depot_update(Version::V2, json!({"id":"p1","patch":{"note":"fragile-two","handling_note":"keep cold"}}), head.clone(), |request, context, stored| {
        assert_eq!(context.original_version, Version::V2);
        assert_eq!(stored.handling_note.as_deref(), Some("keep cold"), "the retired patch member must reach the handler");
        let patched = {
            let mut current = resource.lock().unwrap();
            current.note = request.patch.note.clone();
            current.clone()
        };
        update_notes.lock().unwrap().insert(request.id.clone(), stored.handling_note.clone().unwrap());
        update_writes.fetch_add(1, Ordering::SeqCst);
        let stored = Storage { handling_notes: update_notes.lock().unwrap().clone(), ..Storage::default() };
        async move { HeadOutcome { outcome: Ok::<_, v3::DepotUpdateError>(patched), stored } }
    }, &hooks).await.unwrap() {
        ProjectionOutcome::Projected(DepotUpdateOutcome::V2(Ok(parcel))) => {
            assert_eq!(parcel.handling_note.as_deref(), Some("keep cold"), "the persisted member must read back through the response chain");
            assert_eq!(parcel.note.as_deref(), Some("fragile-two"));
        }, _ => panic!("the v2 update must project"),
    }
    assert_eq!(update_writes.load(Ordering::SeqCst), 1);
    // Read then an unchanged write: patching with exactly the served values
    // projects to the identical v2 view.
    let before = match downcast_parcel(Version::V2, resource.lock().unwrap().clone(), &storage, &hooks).unwrap() {
        ParcelProjection::V2(value) => value,
        _ => panic!("the shared resource must project to v2"),
    };
    let unchanged = dispatch_depot_update(Version::V2, json!({"id":"p1","patch":{"note":before.note,"handling_note":before.handling_note}}), head.clone(), |request, _, stored| {
        let patched = {
            let mut current = resource.lock().unwrap();
            current.note = request.patch.note.clone();
            current.clone()
        };
        update_notes.lock().unwrap().insert(request.id.clone(), stored.handling_note.unwrap_or_default());
        update_writes.fetch_add(1, Ordering::SeqCst);
        let stored = Storage { handling_notes: update_notes.lock().unwrap().clone(), ..Storage::default() };
        async move { HeadOutcome { outcome: Ok::<_, v3::DepotUpdateError>(patched), stored } }
    }, &hooks).await.unwrap();
    match unchanged {
        ProjectionOutcome::Projected(DepotUpdateOutcome::V2(Ok(parcel))) => {
            assert_eq!(parcel.note, before.note);
            assert_eq!(parcel.handling_note, before.handling_note);
        }, _ => panic!("an unchanged write must project identically"),
    }
    assert_eq!(update_writes.load(Ordering::SeqCst), 2);
    // The Head arm carries an empty bag: no step between a v3 request and
    // Head ever holds a retired member, so there is nothing to persist.
    match dispatch_depot_update(Version::V3, json!({"id":"p1","patch":{}}), head.clone(), |_, context, stored| {
        assert_eq!(context.original_version, Version::V3);
        assert!(stored.handling_note.is_none(), "Head callers must carry an empty bag");
        update_writes.fetch_add(1, Ordering::SeqCst);
        let patched = resource.lock().unwrap().clone();
        let stored = Storage { handling_notes: update_notes.lock().unwrap().clone(), ..Storage::default() };
        async move { HeadOutcome { outcome: Ok::<_, v3::DepotUpdateError>(patched), stored } }
    }, &hooks).await.unwrap() {
        ProjectionOutcome::Projected(DepotUpdateOutcome::V3(Ok(_))) => {},
        _ => panic!("the Head update must dispatch"),
    }
    assert_eq!(update_writes.load(Ordering::SeqCst), 3);
    // A business refusal before the handler commits nothing.
    let refusing = Hooks { reject_patch: true, ..Hooks::default() };
    assert!(matches!(
        dispatch_depot_update(Version::V2, json!({"id":"p1","patch":{"handling_note":"lost"}}), head.clone(), refused_handler, &refusing).await,
        Err(AdaptationError::Unsupported(_))
    ));
    assert_eq!(update_writes.load(Ordering::SeqCst), 3);
    // Wire validation still rules the envelope before any hook or handler.
    for (version, wire) in [
        (Version::V3, json!({"id":"p1","patch":{"handling_note":"gone"}})),
        (Version::V2, json!({"id":"p1","patch":{"carrier":"postal"}})),
        (Version::V1, json!({"patch":{"note":"x"}})),
    ] {
        assert!(matches!(
            dispatch_depot_update(version, wire, head.clone(), refused_handler, &hooks).await,
            Err(AdaptationError::Validation(_))
        ), "{version:?} must reject the invalid update envelope");
    }
    assert_eq!(update_writes.load(Ordering::SeqCst), 3);
    // The derived type keeps one flattened wire shape: the whole conversion
    // is a single explicit decision, so the hook reads storage itself and an
    // unarchived label omits the retired member instead of inventing one.
    hooks.signed_tags.lock().unwrap().insert("proof".into(), "archived".into());
    let receipt = |outcome: v3::Signed| {
        let hooks = &hooks;
        async move {
            match dispatch_depot_receipt(Version::V1, json!({"parcel": parcel()}), |_, _| async {
                Ok(outcome)
            }, hooks).await.unwrap() {
                ProjectionOutcome::Projected(DepotReceiptOutcome::V1(Ok(value))) => value,
                _ => panic!("the v1 receipt must project"),
            }
        }
    };
    let archived = receipt(v3::Signed { base: v3::Labelled { label: "proof".into() }, signature: "sig-1".into() }).await;
    assert_eq!(archived.base.tag.as_deref(), Some("archived"));
    assert_eq!(archived.signature, "sig-1");
    let fresh = receipt(v3::Signed { base: v3::Labelled { label: "fresh".into() }, signature: "sig-2".into() }).await;
    assert_eq!(fresh.base.tag, None, "an unarchived label omits the member");
    assert_eq!(hooks.signed.load(Ordering::SeqCst), 2);
    let sealed = v3::Signed { base: v3::Labelled { label: "sealed-label".into() }, signature: "sig-3".into() };
    match dispatch_depot_receipt(Version::V1, json!({"parcel": parcel()}), |_, _| async {
        Ok(sealed)
    }, &hooks).await.unwrap() {
        ProjectionOutcome::Unsupported { head: Ok(original), error } => {
            assert_eq!(original.base.label, "sealed-label", "the refusal retains the committed Head outcome");
            assert_eq!(error.type_path, "Signed.tag");
        },
        _ => panic!("a sealed label must refuse the receipt"),
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
