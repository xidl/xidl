#[allow(warnings)]
mod gen {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/versioned_events_rust.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/versioned_events_rust.adapters.rs"));
}
mod events;

fn main() {
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(events::run());
    println!("PASS: event split keeps old names, distinct ids and one stated half");
}
