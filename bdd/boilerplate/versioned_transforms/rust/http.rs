//! The Rust transform-shape journey over real HTTP: the same negotiation,
//! projection, refusal and round-trip claims the TypeScript journey pins.

use crate::gen::v2026_07_01;
use crate::http_app::{directory_router, AppState};
use axum::http::StatusCode;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

/// Every transform shape behaves identically over the wire.
pub async fn run_http_journey() {
    let app = Arc::new(AppState::default());
    let router = directory_router(app.clone());
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
    let post = |version: Option<&str>, body: Value| {
        let http = http.clone();
        let base = base.clone();
        let version = version.map(str::to_owned);
        let url = format!("{base}/profiles");
        async move {
            let mut request = http.post(url);
            if let Some(version) = version {
                request = request.header("x-transform-version", version);
            }
            request.json(&body).send().await.unwrap()
        }
    };
    let get = |version: Option<&str>, id: &str| {
        let http = http.clone();
        let base = base.clone();
        let version = version.map(str::to_owned);
        let id = id.to_owned();
        let url = format!("{base}/profiles?id={id}");
        async move {
            let mut request = http.get(url);
            if let Some(version) = version {
                request = request.header("x-transform-version", version);
            }
            request.send().await.unwrap()
        }
    };

    // Negotiation: an impossible calendar date is a 400, never a guess.
    for probe in ["2026-02-30", "2026-13-01", "nope"] {
        let response = get(Some(probe), "missing").await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{}", probe);
        assert_eq!(
            response.headers().get("x-transform-version").unwrap(),
            "unresolved"
        );
    }

    // The oldest release publishes; the response is exactly its own contract.
    let oldest = json!({
        "created_at": 1_700_000_000i64,
        "first_name": "Ann",
        "id": "p1",
        "last_name": "Lee",
        "note": { "text": "prefers email" },
        "recipient": "anna@example.test",
        "status": "sent",
        "title": "founder",
    });
    let published = post(Some("2026-01-01"), oldest.clone()).await;
    assert_eq!(published.status(), StatusCode::OK);
    assert_eq!(published.json::<Value>().await.unwrap(), oldest);
    assert_eq!(app.writes.load(Ordering::SeqCst), 1);

    // Head reads the merged view: the stated fallback, the structured route
    // without an invented protocol, and the deterministic RFC 3339 stamp.
    let head_view: Value = get(None, "p1").await.json().await.unwrap();
    assert_eq!(head_view["display_name"], json!("Ann Lee"));
    assert_eq!(head_view["fallback"], json!(true));
    assert_eq!(head_view["to"], json!({ "handle": "anna@example.test" }));
    assert_eq!(head_view["created_at_utc"], json!("2023-11-14T22:13:20.000Z"));
    assert_eq!(head_view["notes"], json!([{ "text": "prefers email" }]));
    assert!(head_view.get("status").is_none());

    // The middle release reads back its own contract.
    let v2_view: Value = get(Some("2026-04-01"), "p1").await.json().await.unwrap();
    assert_eq!(v2_view["created_at"], json!(1_700_000_000i64));
    assert_eq!(v2_view["fallback"], json!(true));
    assert_eq!(v2_view["status"], json!("sent"));
    assert_eq!(v2_view["to"], json!({ "handle": "anna@example.test" }));

    // Round-trip rule one: the oldest caller reads exactly what it wrote.
    let again: Value = get(Some("2026-01-01"), "p1").await.json().await.unwrap();
    assert_eq!(again, oldest);

    // E5 upward over HTTP: a retired enum value is refused before the handler
    // runs, so nothing is committed.
    let refused = post(
        Some("2026-01-01"),
        json!({
            "created_at": 1_700_000_000i64,
            "first_name": "Ann",
            "id": "p1",
            "last_name": "Lee",
            "note": { "text": "prefers email" },
            "recipient": "anna@example.test",
            "status": "read",
            "title": "founder",
        }),
    )
    .await;
    assert_eq!(refused.status(), StatusCode::BAD_REQUEST);
    assert_eq!(app.writes.load(Ordering::SeqCst), 1);

    // The middle release publishes; the fallback it states stays exactly as
    // stated on the Head view.
    let announced = post(
        Some("2026-04-01"),
        json!({
            "created_at": 1_770_000_000i64,
            "fallback": false,
            "first_name": "Ann",
            "id": "p2",
            "last_name": "Lee",
            "note": { "text": "morning person" },
            "status": "delivered",
            "title": "engineer",
            "to": { "handle": "lee@example.test", "protocol": "relay" },
        }),
    )
    .await;
    assert_eq!(announced.status(), StatusCode::OK);
    assert_eq!(app.writes.load(Ordering::SeqCst), 2);
    let announced_head: Value = get(None, "p2").await.json().await.unwrap();
    assert_eq!(announced_head["display_name"], json!("Ann Lee"));
    assert_eq!(announced_head["fallback"], json!(false));

    // The Head rewrite customises the display, grows the notes and switches
    // the route to the newest protocol value with a read time.
    let rewritten = post(
        None,
        json!({
            "created_at_utc": "2026-06-01T12:00:00.000Z",
            "display_name": "Dr. Ann Lee",
            "id": "p2",
            "notes": [{ "text": "morning person" }, { "text": "vip" }],
            "read_at": "2026-06-02T08:00:00.000Z",
            "title": "engineer",
            "to": { "handle": "lee@example.test", "protocol": "chat" },
        }),
    )
    .await;
    assert_eq!(rewritten.status(), StatusCode::OK);
    assert_eq!(app.writes.load(Ordering::SeqCst), 3);
    let rewritten_head: Value = get(None, "p2").await.json().await.unwrap();
    assert_eq!(rewritten_head["display_name"], json!("Dr. Ann Lee"));
    // fallback is a live Head member: the rewriter sees it and owns it, so
    // omitting it clears it instead of resurrecting the stored value.
    assert_eq!(rewritten_head["fallback"], Value::Null);

    // E4: the newest value has no image in any release that still declares
    // the field; both older callers are refused at the first such step.
    for probe in ["2026-04-01", "2026-01-01"] {
        let chat_refusal = get(Some(probe), "p2").await;
        assert_eq!(chat_refusal.status(), StatusCode::UNPROCESSABLE_ENTITY, "{}", probe);
        let body: Value = chat_refusal.json().await.unwrap();
        assert!(body["reason"].as_str().unwrap_or_default().contains("chat"), "{}", probe);
    }

    // E3 write rules: restating the stored names keeps the custom display;
    // changing the names recomputes the mechanical merge.
    let restated = post(
        Some("2026-04-01"),
        json!({
            "created_at": 1_770_000_000i64,
            "first_name": "Ann",
            "id": "p2",
            "last_name": "Lee",
            "status": "delivered",
            "title": "engineer",
            "to": { "handle": "lee@example.test", "protocol": "relay" },
        }),
    )
    .await;
    assert_eq!(restated.status(), StatusCode::OK);
    let restated_head: Value = get(None, "p2").await.json().await.unwrap();
    assert_eq!(restated_head["display_name"], json!("Dr. Ann Lee"));
    let renamed = post(
        Some("2026-04-01"),
        json!({
            "created_at": 1_770_000_000i64,
            "first_name": "Anna",
            "id": "p2",
            "last_name": "Lee",
            "status": "delivered",
            "title": "engineer",
            "to": { "handle": "lee@example.test", "protocol": "relay" },
        }),
    )
    .await;
    assert_eq!(renamed.status(), StatusCode::OK);
    let renamed_head: Value = get(None, "p2").await.json().await.unwrap();
    assert_eq!(renamed_head["display_name"], json!("Anna Lee"));

    // E7 write rules: a stated note replaces the visible head and keeps the
    // entries it cannot see; an omitted note changes nothing.
    let single_note = post(
        Some("2026-04-01"),
        json!({
            "created_at": 1_770_000_000i64,
            "first_name": "Anna",
            "id": "p2",
            "last_name": "Lee",
            "note": { "text": "call first" },
            "status": "delivered",
            "title": "engineer",
            "to": { "handle": "lee@example.test", "protocol": "relay" },
        }),
    )
    .await;
    assert_eq!(single_note.status(), StatusCode::OK);
    let single_head: Value = get(None, "p2").await.json().await.unwrap();
    assert_eq!(
        single_head["notes"],
        json!([{ "text": "call first" }, { "text": "vip" }])
    );
    let silent = post(
        Some("2026-04-01"),
        json!({
            "created_at": 1_770_000_000i64,
            "first_name": "Anna",
            "id": "p2",
            "last_name": "Lee",
            "status": "delivered",
            "title": "engineer",
            "to": { "handle": "lee@example.test", "protocol": "relay" },
        }),
    )
    .await;
    assert_eq!(silent.status(), StatusCode::OK);
    let silent_head: Value = get(None, "p2").await.json().await.unwrap();
    assert_eq!(
        silent_head["notes"],
        json!([{ "text": "call first" }, { "text": "vip" }])
    );
    let v2_note: Value = get(Some("2026-04-01"), "p2").await.json().await.unwrap();
    assert_eq!(v2_note["note"], json!({ "text": "call first" }));

    // A profile that only ever existed at Head cannot serve the pieces it
    // never had: the display name cannot be split, so the read refuses.
    let ghost = post(
        None,
        json!({
            "created_at_utc": "2026-06-01T09:00:00.000Z",
            "display_name": "Solo",
            "id": "p3",
            "notes": [],
            "title": "ghost",
            "to": { "handle": "ghost@example.test" },
        }),
    )
    .await;
    assert_eq!(ghost.status(), StatusCode::OK);
    assert_eq!(get(Some("2026-04-01"), "p3").await.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let _ = v2026_07_01::RouteProtocol::sms;
    stop.send(()).unwrap();
    server.await.unwrap();
    println!("PASS: transform shapes negotiate, project, refuse and round-trip over HTTP");
}
