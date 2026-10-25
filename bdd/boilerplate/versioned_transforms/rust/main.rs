#[allow(warnings)]
mod gen {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/versioned_transforms_rust.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/versioned_transforms_rust.adapters.rs"));
}
mod hooks;
mod routes;
mod http;
mod http_app;
mod http_roster;
mod transforms;

fn main() {
    transforms::assert_transforms();
    println!("PASS: transform shapes upcast, downcast and round-trip");
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(http::run_http_journey());
}
