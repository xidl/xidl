pub mod gen {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/versioned_bounds.rs"));
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/versioned_bounds.adapters.rs"
    ));
}

use gen::version_adapters::{
    self as a, AdaptationError, BatchesReserveOutcome, BatchesUploadOutcome, ProjectionOutcome,
    Version,
};
use serde_json::{json, Value};
use std::cell::Cell;

struct Hooks;
impl a::VersionAdapters for Hooks {}

fn batch(version: Version, count: usize) -> Value {
    let invoices = (0..count)
        .map(|id| {
            let mut invoice = json!({"id": id});
            if version != Version::V1 {
                invoice["memo"] = json!("payment received");
            }
            if version == Version::V3 {
                invoice["label"] = json!("approved");
            }
            invoice
        })
        .collect::<Vec<_>>();
    json!({"invoices": invoices, "regions": {"eu": invoices, "us": []},
           "days": [invoices, []], "retry": []})
}

#[tokio::main]
async fn main() {
    let calls = Cell::new(0);
    for version in [Version::V1, Version::V2, Version::V3] {
        for count in 0..=2 {
            let wire = batch(version, count);
            let result = a::dispatch_batches_reserve(
                version,
                json!({"batch": wire}),
                |request, context| {
                    calls.set(calls.get() + 1);
                    assert_eq!(context.original_version, version);
                    assert_eq!(request.batch.invoices.len(), count);
                    assert!(request
                        .batch
                        .invoices
                        .iter()
                        .all(|invoice| (version != Version::V1 || invoice.memo.is_none())
                            && (version == Version::V3 || invoice.label.is_none())));
                    async move { Ok(request.batch) }
                },
                &Hooks,
            )
            .await
            .unwrap();
            let returned = match result {
                ProjectionOutcome::Projected(BatchesReserveOutcome::V1(Ok(value))) => {
                    serde_json::to_value(value).unwrap()
                }
                ProjectionOutcome::Projected(BatchesReserveOutcome::V2(Ok(value))) => {
                    serde_json::to_value(value).unwrap()
                }
                ProjectionOutcome::Projected(BatchesReserveOutcome::V3(Ok(value))) => {
                    serde_json::to_value(value).unwrap()
                }
                _ => panic!("valid invoice batch must project to the caller's version"),
            };
            assert_eq!(
                returned, wire,
                "projection must not pad, truncate or leak future members"
            );
        }
        let valid = batch(version, 0);
        for (field, value) in [
            ("invoices", json!([{"id":1}, {"id":2}, {"id":3}])),
            ("regions", json!({"eu":[], "us":[], "jp":[]})),
            ("regions", json!({"eu":[{"id":1}, {"id":2}, {"id":3}]})),
            ("days", json!([[{"id":1}, {"id":2}, {"id":3}], []])),
            ("days", json!([[]])),
            ("retry", json!([{"id":1}, {"id":2}, {"id":3}])),
            ("invoices", json!([{"id":"1"}])),
        ] {
            let mut invalid = valid.clone();
            invalid[field] = value;
            let before = calls.get();
            let result = a::dispatch_batches_reserve(
                version,
                json!({"batch": invalid}),
                |request, _| {
                    calls.set(calls.get() + 1);
                    async move { Ok(request.batch) }
                },
                &Hooks,
            )
            .await;
            assert!(
                matches!(result, Err(AdaptationError::Validation(_))),
                "{field}"
            );
            assert_eq!(
                calls.get(),
                before,
                "invalid wire data must not reach the handler"
            );
        }
        for count in 0..=3 {
            let before = calls.get();
            let result = a::dispatch_batches_upload(
                version,
                json!({"bytes": vec![1_u8; count]}),
                |request, _| {
                    calls.set(calls.get() + 1);
                    async move { Ok(request.bytes.len() as i32) }
                },
                &Hooks,
            )
            .await;
            if count > 2 {
                assert!(matches!(result, Err(AdaptationError::Validation(_))));
                assert_eq!(calls.get(), before);
            } else {
                let returned = match result.unwrap() {
                    ProjectionOutcome::Projected(BatchesUploadOutcome::V1(Ok(value)))
                    | ProjectionOutcome::Projected(BatchesUploadOutcome::V2(Ok(value)))
                    | ProjectionOutcome::Projected(BatchesUploadOutcome::V3(Ok(value))) => value,
                    _ => panic!("bounded bytes must project to the caller's version"),
                };
                assert_eq!(returned as usize, count);
            }
        }
    }
    assert_eq!(calls.get(), 18);
    println!("PASS: bounded projections preserve three frozen contracts");
}
