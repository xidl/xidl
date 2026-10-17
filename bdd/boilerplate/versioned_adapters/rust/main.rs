#[allow(warnings)]
mod gen {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/versioned_adapters_rust_business.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/versioned_adapters_rust_business.adapters.rs"));
}
mod hooks;
mod journey;
mod wire;

#[tokio::main]
async fn main() {
    wire::run().await;
    journey::run().await;
    println!("PASS: Rust three-version wire, ownership, side effects and errors");
}
