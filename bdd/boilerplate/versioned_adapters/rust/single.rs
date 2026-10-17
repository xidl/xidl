use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
#[allow(warnings)]
mod gen {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/versioned_adapters_single.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/versioned_adapters_single.adapters.rs"));
}
use gen::version_adapters::*;
struct Hooks;
impl VersionAdapters for Hooks {}
#[tokio::main]
async fn main() {
    let calls = AtomicUsize::new(0);
    let outcome = dispatch_depot_ship(Version::Only, json!({"parcel":{"id":"p"}}), |input, context| {
        calls.fetch_add(1, Ordering::SeqCst); assert_eq!(context.original_version,Version::Only);
        assert!(input.parcel.note.is_none()); async { Ok(input.parcel) }
    }, &Hooks).await.unwrap();
    match outcome {
        ProjectionOutcome::Projected(DepotShipOutcome::Only(Ok(value))) => {
            let pointer = value.id.as_ptr();
            match downcast_parcel(Version::Only,value,&Hooks).unwrap() {
                ParcelProjection::Only(value) => assert_eq!(value.id.as_ptr(),pointer),
            }
        }, _ => panic!("single-version identity must succeed"),
    }
    for wire in [json!({"parcel":{}}), json!({"parcel":{"id":"p","future":true}}), json!({"parcel":{"id":2}})] {
        let outcome = dispatch_depot_ship(Version::Only,wire,|input,_| { calls.fetch_add(1, Ordering::SeqCst); async { Ok(input.parcel) } },&Hooks).await;
        assert!(matches!(outcome,Err(AdaptationError::Validation(_))));
    }
    assert_eq!(calls.load(Ordering::SeqCst),1);
    println!("PASS: Rust single-version validation and owned identity");
}
