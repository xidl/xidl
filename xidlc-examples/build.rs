use xidl_build::Builder;

fn build_openapi(file_name: &str) {
    Builder::new()
        .with_lang("rust-axum")
        .compile(&[format!("./api/http/{file_name}.idl")])
        .expect("failed to compile city http idl for rust-axum example");
    Builder::new()
        .with_lang("openapi")
        .with_out_dir("./api/http/generated/")
        .with_output_filename(format!("{file_name}.json"))
        .compile(&[format!("./api/http/{file_name}.idl")])
        .expect("failed to compile city http stream openapi");
}

fn build_jsonrpc(file_name: &str) {
    Builder::new()
        .with_lang("rust-jsonrpc")
        .compile(&[format!("./api/jsonrpc/{file_name}.idl")])
        .expect("failed to compile city jsonrpc idl for rust-jsonrpc example");
    Builder::new()
        .with_lang("openrpc")
        .with_out_dir("./api/jsonrpc/generated/")
        .with_output_filename(format!("{file_name}_openrpc.json"))
        .compile(&[format!("./api/jsonrpc/{file_name}.idl")])
        .expect("failed to compile city jsonrpc stream openrpc");
}

fn main() {
    println!("cargo:rerun-if-changed=./examples");
    println!("cargo:rerun-if-changed=./api");
    println!("cargo:rerun-if-changed=build.rs");

    for file in glob::glob("./api/http/*.idl").expect("failed to read glob pattern") {
        let path = file.expect("failed to read path from glob");
        let file_name = path
            .file_stem()
            .expect("failed to get file stem")
            .to_str()
            .expect("failed to convert file stem to string");
        build_openapi(file_name);
    }

    for file in glob::glob("./api/jsonrpc/*.idl").expect("failed to read glob pattern") {
        let path = file.expect("failed to read path from glob");
        let file_name = path
            .file_stem()
            .expect("failed to get file stem")
            .to_str()
            .expect("failed to convert file stem to string");
        build_jsonrpc(file_name);
    }
}
