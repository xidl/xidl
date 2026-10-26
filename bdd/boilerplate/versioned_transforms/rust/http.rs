//! The Rust transform-shape journey over real HTTP, part one: the same
//! negotiation, publication and caller-aware rewrite claims the TypeScript
//! journey pins. The directory-wide read legs live in http_roster.rs.

use crate::http_app::{directory_router, AppState, JourneyClient};
use axum::http::StatusCode;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::{atomic::Ordering, Arc};

/// Every transform shape behaves identically over the wire.
pub async fn run_http_journey() {
    let app = Arc::new(AppState::default());
    let router = directory_router(app.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    let client = JourneyClient::new(format!("http://{addr}"));

    // Negotiation: an impossible calendar date is a 400, never a guess.
    for probe in ["2026-02-30", "2026-13-01", "nope"] {
        let response = client.get(Some(probe), "missing").await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{}", probe);
        assert_eq!(
            response.headers().get("x-transform-version").unwrap(),
            "unresolved"
        );
    }

    // A read of an absent profile is a typed 404 at every served version.
    for (probe, echoed) in [
        (None, "2026-07-01"),
        (Some("2026-04-01"), "2026-04-01"),
        (Some("2026-01-01"), "2026-01-01"),
    ] {
        let absent = client.get(probe, "nope").await;
        assert_eq!(absent.status(), StatusCode::NOT_FOUND, "{}", echoed);
        assert_eq!(
            absent.headers().get("x-transform-version").unwrap(),
            echoed
        );
    }

    // An empty directory refuses the roster read; a refusal never becomes a
    // successful empty list.
    let empty_roster = client.roster(Some("2026-04-01")).await;
    assert_eq!(empty_roster.status(), StatusCode::INTERNAL_SERVER_ERROR);

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
    let published = client.post(Some("2026-01-01"), oldest.clone()).await;
    assert_eq!(published.status(), StatusCode::OK);
    assert_eq!(published.json::<Value>().await.unwrap(), oldest);
    assert_eq!(app.writes.load(Ordering::SeqCst), 1);

    // Head reads the merged view: the stated fallback, the structured route
    // without an invented protocol, and the deterministic RFC 3339 stamp.
    let head_view: Value = client.get(None, "p1").await.json().await.unwrap();
    assert_eq!(head_view["display_name"], json!("Ann Lee"));
    assert_eq!(head_view["fallback"], json!(true));
    assert_eq!(head_view["to"], json!({ "handle": "anna@example.test" }));
    assert_eq!(head_view["created_at_utc"], json!("2023-11-14T22:13:20.000Z"));
    assert_eq!(head_view["tier"], json!("standard"));
    assert_eq!(head_view["notes"], json!([{ "text": "prefers email" }]));
    assert!(head_view.get("status").is_none());

    // The middle release reads back its own contract.
    let v2_view: Value = client
        .get(Some("2026-04-01"), "p1")
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(v2_view["created_at"], json!(1_700_000_000i64));
    assert_eq!(v2_view["fallback"], json!(true));
    assert_eq!(v2_view["status"], json!("sent"));
    assert_eq!(v2_view["to"], json!({ "handle": "anna@example.test" }));

    // Round-trip rule one: the oldest caller reads exactly what it wrote.
    let again: Value = client
        .get(Some("2026-01-01"), "p1")
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(again, oldest);

    // E5 upward over HTTP: a retired enum value is refused before the handler
    // runs, so nothing is committed.
    let refused = client
        .post(
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
    let announced = client
        .post(
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
    let announced_head: Value = client.get(None, "p2").await.json().await.unwrap();
    assert_eq!(announced_head["display_name"], json!("Ann Lee"));
    assert_eq!(announced_head["fallback"], json!(false));

    // The Head rewrite customises the display, grows the notes and switches
    // the route to the newest protocol value with a read time.
    let rewritten = client
        .post(
            None,
            json!({
                "created_at_utc": "2026-06-01T12:00:00.000Z",
                "display_name": "Dr. Ann Lee",
                "id": "p2",
                "notes": [{ "text": "morning person" }, { "text": "vip" }],
                "read_at": "2026-06-02T08:00:00.000Z",
                "tier": "premium",
                "title": "engineer",
                "to": { "handle": "lee@example.test", "protocol": "chat" },
            }),
        )
        .await;
    assert_eq!(rewritten.status(), StatusCode::OK);
    assert_eq!(app.writes.load(Ordering::SeqCst), 3);
    let rewritten_head: Value = client.get(None, "p2").await.json().await.unwrap();
    assert_eq!(rewritten_head["display_name"], json!("Dr. Ann Lee"));
    // fallback is a live Head member: the rewriter sees it and owns it, so
    // omitting it clears it instead of resurrecting the stored value.
    assert_eq!(rewritten_head["fallback"], Value::Null);

    // E4: the newest value has no image in any release that still declares
    // the field; both older callers are refused at the first such step.
    for probe in ["2026-04-01", "2026-01-01"] {
        let chat_refusal = client.get(Some(probe), "p2").await;
        assert_eq!(chat_refusal.status(), StatusCode::UNPROCESSABLE_ENTITY, "{}", probe);
        let body: Value = chat_refusal.json().await.unwrap();
        assert!(body["reason"].as_str().unwrap_or_default().contains("chat"), "{}", probe);
    }

    // E3 write rules: restating the stored names keeps the custom display;
    // changing the names recomputes the mechanical merge.
    let restated = client
        .post(
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
    let restated_head: Value = client.get(None, "p2").await.json().await.unwrap();
    assert_eq!(restated_head["display_name"], json!("Dr. Ann Lee"));
    // E8, rule two through the upcast: a v2 write that omits the flag leaves
    // it absent at Head; the default never sneaks back in.
    assert!(restated_head.get("fallback").is_none());
    let renamed = client
        .post(
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
    let renamed_head: Value = client.get(None, "p2").await.json().await.unwrap();
    assert_eq!(renamed_head["display_name"], json!("Anna Lee"));

    // E7 write rules: a stated note replaces the visible head and keeps the
    // entries it cannot see; an omitted note changes nothing.
    let single_note = client
        .post(
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
    let single_head: Value = client.get(None, "p2").await.json().await.unwrap();
    assert_eq!(
        single_head["notes"],
        json!([{ "text": "call first" }, { "text": "vip" }])
    );
    let silent = client
        .post(
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
    let silent_head: Value = client.get(None, "p2").await.json().await.unwrap();
    assert_eq!(
        silent_head["notes"],
        json!([{ "text": "call first" }, { "text": "vip" }])
    );
    let v2_note: Value = client
        .get(Some("2026-04-01"), "p2")
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(v2_note["note"], json!({ "text": "call first" }));

    // E10: the refinement raised at Head reaches the middle release through
    // the downgrade decision, keeping the committed status instead of being
    // translated into a missing profile.
    app.sealed.lock().unwrap().insert("p1".into());
    let sealed_v2 = client.get(Some("2026-04-01"), "p1").await;
    assert_eq!(sealed_v2.status(), StatusCode::CONFLICT);
    let sealed_body: Value = sealed_v2.json().await.unwrap();
    assert!(sealed_body["reason"].as_str().unwrap_or_default().contains("sealed"));
    let sealed_head = client.get(None, "p1").await;
    assert_eq!(sealed_head.status(), StatusCode::CONFLICT);
    app.sealed.lock().unwrap().remove("p1");

    crate::http_roster::run_roster_legs(&client).await;

    stop.send(()).unwrap();
    server.await.unwrap();
    println!("PASS: transform shapes negotiate, project, refuse and round-trip over HTTP");
}
