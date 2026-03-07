use xidl_build::Builder;

fn main() {
    println!("cargo:rerun-if-changed=../xidlc/**/*.j2");
    println!("cargo:rerun-if-changed=./examples");
    println!("cargo:rerun-if-changed=build.rs");

    Builder::new()
        .with_lang("rust-axum")
        .compile(&["./api/hello_world.idl"])
        .expect("failed to compile idl for rust-axum example");

    Builder::new()
        .with_lang("openapi")
        .with_out_dir("./api/")
        .compile(&["./api/hello_world.idl"])
        .expect("failed to compile idl for rust-axum example");

    Builder::new()
        .with_lang("rust-jsonrpc")
        .compile(&["./api/hello_world_jsonrpc.idl"])
        .expect("failed to compile idl for rust-jsonrpc example");

    Builder::new()
        .with_lang("openrpc")
        .with_out_dir("./api")
        .compile(&["./api/hello_world_jsonrpc.idl"])
        .expect("failed to compile idl for rust-jsonrpc example");
}
