//! The Rust booking chain over real HTTP: the same negotiation, projection,
//! idempotency, storage and behaviour claims the TypeScript journey pins,
//! driven through the generated Rust dispatchers on an axum server.

use crate::http_app::{booking_router, AppState};
use axum::http::StatusCode;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

/// The Rust HTTP journey: every claim the TypeScript server journey pins,
/// re-driven through the generated Rust chain.
pub async fn run_http_journey() {
    let app = Arc::new(AppState::default());
    let router = booking_router(app.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    let http = xidl_rust_axum::reqwest::Client::new();
    let post = |version: Option<&str>, body: Value, key: Option<&str>| {
        let http = http.clone();
        let base = base.clone();
        let version = version.map(str::to_owned);
        let key = key.map(str::to_owned);
        let url = format!("{base}/bookings");
        async move {
            let mut request = http.post(url);
            if let Some(version) = version {
                request = request.header("x-booking-version", version);
            }
            if let Some(key) = key {
                request = request.header("idempotency-key", key);
            }
            request.json(&body).send().await.unwrap()
        }
    };
    let get = |version: Option<&str>, id: &str| {
        let http = http.clone();
        let base = base.clone();
        let version = version.map(str::to_owned);
        let id = id.to_owned();
        let url = format!("{base}/bookings?id={id}");
        async move {
            let mut request = http.get(url);
            if let Some(version) = version {
                request = request.header("x-booking-version", version);
            }
            request.send().await.unwrap()
        }
    };

    // The oldest release writes; the response projects back to its own
    // contract and echoes the resolved version.
    let v1_body = json!({
        "agenda": "Kickoff;60",
        "id": "b1",
        "roster": ["user-1"],
        "title": "Kickoff",
    });
    let oldest = post(Some("2026-06-01"), v1_body.clone(), Some("kickoff")).await;
    assert_eq!(oldest.status(), StatusCode::OK);
    assert_eq!(oldest.headers().get("x-booking-version").unwrap(), "2026-06-01");
    let oldest_view: Value = oldest.json().await.unwrap();
    assert_eq!(oldest_view["agenda"], json!("Kickoff;60"));
    assert_eq!(app.writes.load(Ordering::SeqCst), 1);
    let head_view: Value = get(None, "b1").await.json().await.unwrap();
    assert_eq!(head_view["items"], json!([{ "minutes": 60, "title": "Kickoff" }]));

    // This application's example policy: mid-patch dates resolve down;
    // earlier, malformed or calendar-impossible headers are a 400.
    let negotiation: Vec<(Option<&str>, &str, u16)> = vec![
        (None, "2027-01-01", 404),
        (Some("2026-08-15"), "2026-06-01", 404),
        (Some("2026-10-01"), "2026-10-01", 404),
        (Some("2026-12-31"), "2026-10-01", 404),
        (Some("2026-05-31"), "unresolved", 400),
        (Some("next-tuesday"), "unresolved", 400),
        (Some("2026-13-01"), "unresolved", 400),
        (Some("2026-02-30"), "unresolved", 400),
        (Some("2026-6-01"), "unresolved", 400),
    ];
    for (probe, echoed, status) in negotiation {
        let response = get(probe, "missing").await;
        assert_eq!(response.status().as_u16(), status, "{probe:?} -> {status}");
        assert_eq!(response.headers().get("x-booking-version").unwrap(), echoed);
    }
    let unresolved = post(Some("2026-05-31"), v1_body.clone(), Some("unresolved")).await;
    assert_eq!(unresolved.status(), StatusCode::BAD_REQUEST);
    assert_eq!(app.writes.load(Ordering::SeqCst), 1);

    // Transport-level refusals never reach a handler.
    let keyless = post(Some("2026-06-01"), v1_body.clone(), None).await;
    assert_eq!(keyless.status(), StatusCode::BAD_REQUEST);
    let malformed = http
        .post(format!("{base}/bookings"))
        .header("x-booking-version", "2026-06-01")
        .header("idempotency-key", "bad-json")
        .body("not-json{")
        .send()
        .await
        .unwrap();
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
    assert_eq!(app.writes.load(Ordering::SeqCst), 1);

    // Cross-release idempotency: the same meaning from any release replays;
    // a different command conflicts.
    let replay = post(
        Some("2027-01-01"),
        json!({ "id": "b1", "items": [{ "minutes": 60, "title": "Kickoff" }], "title": "Kickoff" }),
        Some("kickoff"),
    )
    .await;
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(app.writes.load(Ordering::SeqCst), 1);
    let middle_replay = post(
        Some("2026-10-01"),
        json!({
            "id": "b1",
            "items": [{ "minutes": 60, "title": "Kickoff" }],
            "locked": false,
            "owner": "user-9",
            "roster": ["ignored-by-the-chain"],
            "title": "Kickoff",
        }),
        Some("kickoff"),
    )
    .await;
    assert_eq!(middle_replay.status(), StatusCode::OK);
    let middle_view: Value = middle_replay.json().await.unwrap();
    assert_eq!(middle_view["roster"], json!(["user-1"]));
    assert_eq!(app.writes.load(Ordering::SeqCst), 1);
    let roster_replay = post(
        Some("2026-06-01"),
        json!({
            "agenda": "Kickoff;60",
            "id": "b1",
            "roster": ["user-9"],
            "title": "Kickoff",
        }),
        Some("kickoff"),
    )
    .await;
    assert_eq!(roster_replay.status(), StatusCode::OK);
    assert_eq!(app.writes.load(Ordering::SeqCst), 1);
    let roster_view: Value = get(Some("2026-06-01"), "b1").await.json().await.unwrap();
    assert_eq!(roster_view["roster"], json!(["user-1"]));
    let conflict = post(
        Some("2027-01-01"),
        json!({ "id": "b1", "items": [{ "minutes": 30, "title": "Changed" }], "title": "Changed" }),
        Some("kickoff"),
    )
    .await;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    assert_eq!(app.writes.load(Ordering::SeqCst), 1);

    // Behaviour switch: Head refuses a blank title; the oldest release accepts
    // it, and replaying the accepted command at Head answers before the gate.
    let blank_v1 = json!({
        "agenda": " ;0",
        "id": "b-blank",
        "roster": [],
        "title": "   ",
    });
    assert_eq!(
        post(Some("2026-06-01"), blank_v1, Some("blank-v1")).await.status(),
        StatusCode::OK
    );
    let blank_replay = post(
        Some("2027-01-01"),
        json!({ "id": "b-blank", "items": [{ "minutes": 0, "title": " " }], "locked": false, "title": "   " }),
        Some("blank-v1"),
    )
    .await;
    assert_eq!(blank_replay.status(), StatusCode::OK);
    assert_eq!(app.writes.load(Ordering::SeqCst), 2);
    let blank_head = post(
        None,
        json!({ "id": "b-blank-head", "items": [{ "minutes": 0, "title": "   " }], "title": "   " }),
        Some("blank-head"),
    )
    .await;
    assert_eq!(blank_head.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(app.writes.load(Ordering::SeqCst), 2);

    // Read your writes, and a Head-created booking refuses both releases that
    // still read a fact it never had.
    let v1_view: Value = get(Some("2026-06-01"), "b1").await.json().await.unwrap();
    assert_eq!(v1_view["agenda"], json!("Kickoff;60"));
    assert_eq!(v1_view["roster"], json!(["user-1"]));
    assert!(v1_view.get("items").is_none());
    assert_eq!(
        post(
            None,
            json!({ "id": "b-head", "items": [{ "minutes": 15, "title": "Ad hoc" }], "title": "Ad hoc" }),
            Some("ad-hoc"),
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(app.writes.load(Ordering::SeqCst), 3);
    for probe in ["2026-06-01", "2026-10-01"] {
        let refused = get(Some(probe), "b-head").await;
        assert_eq!(refused.status(), StatusCode::UNPROCESSABLE_ENTITY, "{}", probe);
        let body: Value = refused.json().await.unwrap();
        assert!(body["reason"].as_str().unwrap_or_default().contains("roster"), "{}", probe);
    }

    // An ambiguous or signed agenda is refused, not guessed; the refusal
    // commits nothing and stores nothing.
    let ambiguous = post(
        Some("2026-06-01"),
        json!({ "agenda": "A;B;45", "id": "b-amb", "roster": [], "title": "Sync" }),
        Some("ambiguous"),
    )
    .await;
    assert_eq!(ambiguous.status(), StatusCode::BAD_REQUEST);
    assert_eq!(app.writes.load(Ordering::SeqCst), 3);
    for agenda in ["Kickoff;60;extra", "Kickoff;-5", "Kickoff;+5", "Kickoff;99999999999999"] {
        let refused = post(
            Some("2026-06-01"),
            json!({ "agenda": agenda, "id": "b-shape", "roster": [], "title": "Sync" }),
            Some("shape"),
        )
        .await;
        assert_eq!(refused.status(), StatusCode::BAD_REQUEST, "{}", agenda);
    }
    assert_eq!(app.writes.load(Ordering::SeqCst), 3);
    assert_eq!(get(Some("2026-06-01"), "b-amb").await.status(), StatusCode::NOT_FOUND);

    // A rewrite never erases the facts the oldest release still reads.
    assert_eq!(
        post(
            Some("2026-06-01"),
            json!({ "agenda": "First;10", "id": "b-dupe", "roster": [], "title": "First" }),
            Some("dupe-1"),
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        post(
            None,
            json!({ "id": "b-dupe", "items": [{ "minutes": 20, "title": "Second" }], "title": "Second" }),
            Some("dupe-2"),
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(app.writes.load(Ordering::SeqCst), 5);
    let dupe_head: Value = get(None, "b-dupe").await.json().await.unwrap();
    assert_eq!(dupe_head["title"], json!("Second"));
    let dupe_v1: Value = get(Some("2026-06-01"), "b-dupe").await.json().await.unwrap();
    assert_eq!(dupe_v1["agenda"], json!("First;10"));
    assert_eq!(dupe_v1["roster"], json!([]));

    // A current-release caller that omits the lockable flag keeps it absent:
    // the implied value is only for callers who cannot state it yet, and a
    // caller who owns the field keeps exactly what it sent.
    let unlocked = post(
        Some("2026-10-01"),
        json!({
            "id": "b-unlocked",
            "items": [{ "minutes": 5, "title": "Open" }],
            "roster": [],
            "title": "Open",
        }),
        Some("unlocked"),
    )
    .await;
    assert_eq!(unlocked.status(), StatusCode::OK);
    let unlocked_view: Value = unlocked.json().await.unwrap();
    assert!(unlocked_view.get("locked").is_none());
    let sealed = post(
        Some("2026-10-01"),
        json!({
            "id": "b-locked",
            "items": [{ "minutes": 6, "title": "Sealed" }],
            "locked": true,
            "roster": [],
            "title": "Sealed",
        }),
        Some("locked"),
    )
    .await;
    assert_eq!(sealed.status(), StatusCode::OK);
    let sealed_view: Value = sealed.json().await.unwrap();
    assert_eq!(sealed_view["locked"], json!(true));
    let sealed_head: Value = get(None, "b-locked").await.json().await.unwrap();
    assert_eq!(sealed_head["locked"], json!(true));

    stop.send(()).unwrap();
    server.await.unwrap();
    println!("PASS: booking chain HTTP journey negotiates, projects, replays and refuses identically");
}
