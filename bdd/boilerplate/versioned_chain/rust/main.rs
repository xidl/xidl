#[allow(warnings)]
mod gen {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/versioned_chain_rust.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/versioned_chain_rust.adapters.rs"));
}
mod chain;
mod hooks;
mod http;
mod http_app;

use crate::gen::version_adapters::Version;

fn main() {
    chain::assert_needs_table();
    let hooks = hooks::Hooks::default();
    let storage = chain::storage();
    chain::assert_conversions(&hooks, &storage);
    // Malformed old-version requests fail validation before any hook runs.
    let invalid = match gen::version_adapters::decode_and_upcast_scheduler_book(
        Version::V20260601,
        serde_json::json!({"booking": {"id": "b2", "title": "No agenda", "roster": []}}),
        &hooks,
    ) {
        Err(gen::version_adapters::AdaptationError::Validation(_)) => true,
        _ => panic!("a booking without its agenda is not valid v2026-06-01 wire data"),
    };
    assert!(invalid, "validation precedes every hook");
    // Two upcasts read the corpus booking; a third proves the rename also
    // runs for a caller who omitted the optional owner.
    assert_eq!(hooks.up_from_v1.load(std::sync::atomic::Ordering::SeqCst), 3);
    println!("PASS: booking chain keeps storage facts and pure conversions");
}
