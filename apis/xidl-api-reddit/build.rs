use xidl_build::Builder;

fn main() {
    println!("cargo:rerun-if-changed=./api");
    println!("cargo:rerun-if-changed=build.rs");

    Builder::new()
        .with_lang("rust-axum")
        .with_client(cfg!(feature = "client"))
        .with_server(true)
        .with_mock(false)
        .compile(&["./api/reddit.idl"])
        .expect("failed to compile reddit for rust-axum");

    if !cfg!(feature = "docs-only") {
        Builder::new()
            .with_lang("openapi")
            .with_out_dir("./api/")
            .compile(&["./api/reddit.idl"])
            .expect("failed to compile reddit openapi");
    }
}
