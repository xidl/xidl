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
        .with_lang("rust-axum")
        .compile(&["./api/city_http.idl"])
        .expect("failed to compile city http idl for rust-axum example");

    Builder::new()
        .with_lang("rust-axum")
        .compile(&["./api/city_http_stream.idl"])
        .expect("failed to compile city http stream idl for rust-axum example");

    Builder::new()
        .with_lang("openapi")
        .with_out_dir("./api/")
        .with_output_filename("city_openapi.json")
        .compile(&["./api/city_http.idl"])
        .expect("failed to compile idl for rust-axum example");

    Builder::new()
        .with_lang("openapi")
        .with_out_dir("./api/")
        .with_output_filename("city_http_stream_openapi.json")
        .compile(&["./api/city_http_stream.idl"])
        .expect("failed to compile city http stream openapi");

    Builder::new()
        .with_lang("rust-jsonrpc")
        .compile(&["./api/hello_world_jsonrpc.idl"])
        .expect("failed to compile idl for rust-jsonrpc example");

    Builder::new()
        .with_lang("rust-jsonrpc")
        .compile(&["./api/city_jsonrpc.idl"])
        .expect("failed to compile city jsonrpc idl for rust-jsonrpc example");

    Builder::new()
        .with_lang("openrpc")
        .with_out_dir("./api")
        .with_output_filename("city_openrpc.json")
        .compile(&["./api/city_jsonrpc.idl"])
        .expect("failed to compile idl for rust-jsonrpc example");
}
