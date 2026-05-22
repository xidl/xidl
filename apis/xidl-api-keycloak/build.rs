use xidl_build::Builder;

fn main() {
    println!("cargo:rerun-if-changed=./api");
    println!("cargo:rerun-if-changed=build.rs");

    Builder::new()
        .with_lang("rust-axum")
        .with_client(cfg!(feature = "client"))
        .with_server(cfg!(feature = "server"))
        .with_mock(false)
        .compile(&["./api/keycloak.idl"])
        .expect("failed to compile keycloak for rust-axum");
    Builder::new()
        .with_lang("openapi")
        .with_out_dir("./api/")
        .compile(&["./api/keycloak.idl"])
        .expect("failed to compile keycloak openapi");
}
