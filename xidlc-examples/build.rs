use std::path::Path;

use xidl_build::Builder;

fn main() {
    let axum_idl = Path::new("examples/hello_world.idl");
    let jsonrpc_idl = Path::new("examples/hello_world_jsonrpc.idl");

    println!("cargo:rerun-if-changed=../xidlc/**/*.j2");
    println!("cargo:rerun-if-changed={}", axum_idl.display());
    println!("cargo:rerun-if-changed={}", jsonrpc_idl.display());
    println!("cargo:rerun-if-changed=build.rs");

    Builder::new()
        .with_lang("rust-axum")
        .compile(&[axum_idl])
        .expect("failed to compile idl for rust-axum example");

    Builder::new()
        .with_lang("rust-jsonrpc")
        .compile(&[jsonrpc_idl])
        .expect("failed to compile idl for rust-jsonrpc example");
}
