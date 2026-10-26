//! The E11 event-split journey: both halves of a retired broad kind serve
//! old subscribers under the old name, each push keeps its own id, and the
//! upcast states the one half that owns the name.

use crate::gen::version_adapters::*;
use crate::gen::v2026_07_01;
use serde_json::json;
use std::sync::Mutex;

#[derive(Default)]
struct Hooks {
    /// Storage only moves forward: every emitted event keeps its exact id.
    events: Mutex<Vec<v2026_07_01::Event>>,
    writes: std::sync::atomic::AtomicUsize,
}

impl VersionAdapters for Hooks {
    fn event_kind_upcast_from_v_2026_04_01(
        &self,
        _source: &EventKindUpcastFromV20260401Changes,
    ) -> VersionedResult<v2026_07_01::EventKind> {
        // The old name keeps one half: the application states which half
        // owns the name; the other half is only reachable at Head.
        Ok(v2026_07_01::EventKind::delivered)
    }
    fn event_kind_downcast_to_v_2026_04_01(
        &self,
        source: &EventKindDowncastToV20260401Changes,
    ) -> VersionedResult<crate::gen::v2026_04_01::EventKind> {
        // Both halves of the split serve the old name: one delivered push
        // and one read push reach an old subscriber as two updated events.
        let _ = source;
        Ok(crate::gen::v2026_04_01::EventKind::updated)
    }
}

pub async fn run() {
fn label(kind: &crate::gen::v2026_07_01::EventKind) -> &'static str {
    match kind {
        crate::gen::v2026_07_01::EventKind::delivered => "delivered",
        crate::gen::v2026_07_01::EventKind::read => "read",
    }
}

fn old_label(kind: &crate::gen::v2026_04_01::EventKind) -> &'static str {
    match kind {
        crate::gen::v2026_04_01::EventKind::updated => "updated",
    }
}

    let hooks = Hooks::default();
    let emit = |version: Version, event: serde_json::Value| {
        let hooks = &hooks;
        async move {
            let result = dispatch_notifier_emit(version, json!({ "event": event }), |request, _| {
                let hooks = &hooks;
                async move {
                    let committed = request.event;
                    hooks.writes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    hooks.events.lock().unwrap().push(committed.clone());
                    Ok::<_, xidl_rust_axum::Error>(committed)
                }
            }, hooks)
            .await
            .unwrap();
            match result {
                ProjectionOutcome::Projected(NotifierEmitOutcome::V20260401(Ok(value))) => {
                    (value.id, old_label(&value.kind).to_string())
                }
                ProjectionOutcome::Projected(NotifierEmitOutcome::V20260701(Ok(value))) => {
                    (value.id, label(&value.kind).to_string())
                }
                _ => panic!("the emitted event must project"),
            }
        }
    };

    // Head emits both halves: the broad name retired, two precise ones born.
    assert_eq!(
        emit(Version::V20260701, json!({"id": "e1", "kind": "delivered", "payload": "receipt ready"})).await,
        ("e1".into(), "delivered".into())
    );
    assert_eq!(
        emit(Version::V20260701, json!({"id": "e2", "kind": "read", "payload": "opened twice"})).await,
        ("e2".into(), "read".into())
    );
    assert_eq!(hooks.writes.load(std::sync::atomic::Ordering::SeqCst), 2);

    // An old subscriber reads the feed: both halves arrive under the retired
    // name, and the two pushes stay distinct through their own ids.
    let old_feed = dispatch_notifier_feed(Version::V20260401, json!({}), |_, _| {
        let hooks = &hooks;
        async move {
            Ok::<_, xidl_rust_axum::Error>(hooks.events.lock().unwrap().clone())
        }
    }, &hooks)
    .await
    .unwrap();
    match old_feed {
        ProjectionOutcome::Projected(NotifierFeedOutcome::V20260401(Ok(value))) => {
            let served: Vec<(String, String)> = value
                .iter()
                .map(|event| (event.id.clone(), old_label(&event.kind).to_string()))
                .collect();
            assert_eq!(
                served,
                vec![
                    ("e1".to_string(), "updated".to_string()),
                    ("e2".to_string(), "updated".to_string()),
                ],
                "one delivered and one read push reach the old subscriber as two updates"
            );
        }
        _ => panic!("the old feed must project"),
    }

    // An old caller emits through the retired name: the upcast states the
    // half that owns the name, and the old response reads the same name back.
    assert_eq!(
        emit(Version::V20260401, json!({"id": "e3", "kind": "updated", "payload": "legacy sender"})).await,
        ("e3".into(), "updated".into())
    );
    let stored = hooks.events.lock().unwrap();
    assert_eq!(stored[2].kind, v2026_07_01::EventKind::delivered, "storage keeps the stated half");
    assert_eq!(
        stored.iter().any(|event| event.id == "e3" && event.kind == v2026_07_01::EventKind::read),
        false,
        "the half that does not own the name is never invented for an old write"
    );
    drop(stored);
    assert_eq!(hooks.writes.load(std::sync::atomic::Ordering::SeqCst), 3);

    // A value the old contract never declared is refused before any hook.
    let refused = dispatch_notifier_emit(
        Version::V20260401,
        json!({ "event": { "id": "e4", "kind": "delivered", "payload": "future name" } }),
        |_, _| async { unreachable!("validation refusals must never reach the handler") },
        &hooks,
    )
    .await;
    assert!(matches!(refused, Err(AdaptationError::Validation(_))));
    assert_eq!(hooks.writes.load(std::sync::atomic::Ordering::SeqCst), 3);

    // Round-trip rule: write through the old name, read back the same name.
    let round_trip = dispatch_notifier_feed(Version::V20260401, json!({}), |_, _| {
        let hooks = &hooks;
        async move { Ok::<_, xidl_rust_axum::Error>(hooks.events.lock().unwrap().clone()) }
    }, &hooks)
    .await
    .unwrap();
    match round_trip {
        ProjectionOutcome::Projected(NotifierFeedOutcome::V20260401(Ok(value))) => {
            assert_eq!(value[2].id, "e3");
            assert_eq!(value[2].kind, crate::gen::v2026_04_01::EventKind::updated);
        }
        _ => panic!("the round trip must project"),
    }
}
