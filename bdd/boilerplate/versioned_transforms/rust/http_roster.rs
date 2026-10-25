//! The Rust transform-shape journey over real HTTP, part two: the
//! directory-wide read legs — the roster projection, the Head-born profile,
//! the list-clearing write and the strict RFC 3339 stamps.

use crate::http_app::JourneyClient;
use axum::http::StatusCode;
use serde_json::{json, Value};

/// E9 and the whole-read refusals: one roster read projects every element
/// with its own facts, and one element without them refuses the entire list.
pub(crate) async fn run_roster_legs(client: &JourneyClient) {
    // E9: one roster read projects every element with its own facts.
    let roster_response = client.roster(Some("2026-04-01")).await;
    assert_eq!(roster_response.status(), StatusCode::OK);
    let roster_v2: Value = roster_response.json().await.unwrap();
    let entries = roster_v2.as_array().unwrap();
    assert_eq!(entries.len(), 2);
    let by_id = |id: &str| {
        entries
            .iter()
            .find(|p| p["id"] == json!(id))
            .cloned()
            .unwrap()
    };
    assert_eq!(by_id("p1")["status"], json!("sent"));
    assert_eq!(by_id("p1")["note"]["text"], json!("prefers email"));
    assert_eq!(by_id("p2")["note"]["text"], json!("call first"));
    assert_eq!(by_id("p2")["created_at"], json!(1_770_000_000i64));
    let roster_v1_response = client.roster(Some("2026-01-01")).await;
    assert_eq!(roster_v1_response.status(), StatusCode::OK);
    let roster_v1: Value = roster_v1_response.json().await.unwrap();
    let mut recipients: Vec<String> = roster_v1
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["recipient"].as_str().unwrap().to_owned())
        .collect();
    recipients.sort();
    assert_eq!(recipients, vec!["anna@example.test", "lee@example.test"]);

    // A profile that only ever existed at Head cannot serve the pieces it
    // never had: the display name cannot be split, so the read refuses.
    let ghost = client
        .post(
            None,
            json!({
                "created_at_utc": "2026-06-01T09:00:00.000Z",
                "display_name": "Solo",
                "id": "p3",
                "notes": [],
                "tier": "standard",
                "title": "ghost",
                "to": { "handle": "ghost@example.test" },
            }),
        )
        .await;
    assert_eq!(ghost.status(), StatusCode::OK);
    let ghost_read = client.get(Some("2026-04-01"), "p3").await;
    assert_eq!(ghost_read.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let ghost_body: Value = ghost_read.json().await.unwrap();
    assert!(
        ghost_body["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("display name"),
        "{}",
        ghost_body
    );

    // E7, the Head-clearing case: a caller that owns the list empties it, and
    // the note fact older callers read is gone with it, never a ghost.
    let cleared = client
        .post(
            None,
            json!({
                "created_at_utc": "2026-06-01T12:00:00.000Z",
                "display_name": "Dr. Ann Lee",
                "id": "p2",
                "notes": [],
                "read_at": "2026-06-02T08:00:00.000Z",
                "tier": "premium",
                "title": "engineer",
                "to": { "handle": "lee@example.test", "protocol": "relay" },
            }),
        )
        .await;
    assert_eq!(cleared.status(), StatusCode::OK);
    let cleared_read = client.get(Some("2026-04-01"), "p2").await;
    assert_eq!(cleared_read.status(), StatusCode::OK);
    let cleared_view: Value = cleared_read.json().await.unwrap();
    assert!(cleared_view.get("note").is_none());
    // E5 downward, end to end: the read time on the Head instance is the
    // newest fact, so the middle release serves the retired value from it.
    assert_eq!(cleared_view["status"], json!("read"));

    // The creation stamp is parsed by one strict RFC 3339 shape in every
    // language: offsets and impossible calendar dates never become facts.
    for stamp in [
        "2026-06-01T12:00:00+02:00",
        "2026-02-31T00:00:00Z",
        "2026-06-01T25:00:00Z",
    ] {
        let stamped = client
            .post(
                None,
                json!({
                    "created_at_utc": stamp,
                    "display_name": "Stamped",
                    "id": "p-stamp",
                    "notes": [],
                    "tier": "standard",
                    "title": "stamp",
                    "to": { "handle": "stamp@example.test" },
                }),
            )
            .await;
        assert_eq!(stamped.status(), StatusCode::OK, "{}", stamp);
        let stamp_read = client.get(Some("2026-04-01"), "p-stamp").await;
        assert_eq!(stamp_read.status(), StatusCode::UNPROCESSABLE_ENTITY, "{}", stamp);
        let body: Value = stamp_read.json().await.unwrap();
        assert!(
            body["reason"]
                .as_str()
                .unwrap_or_default()
                .contains("creation time"),
            "{}",
            body
        );
    }

    // One element without its stored facts refuses the whole read: the list
    // cannot be served partially without inventing the missing pieces. The
    // stamped probe sorts first in the roster, so the whole read refuses on
    // its missing creation time; the unsplittable display name is pinned by
    // the ghost profile's single read above.
    let broken = client.roster(Some("2026-04-01")).await;
    assert_eq!(broken.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = broken.json().await.unwrap();
    assert!(
        body["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("creation time"),
        "{}",
        body
    );
}
