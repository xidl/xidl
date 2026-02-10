use std::{fs, path::Path};

use xidl_build::{Builder, XidlBuild};

fn main() {
    let axum_idl = Path::new("examples/hello_world.idl");
    let jsonrpc_idl = Path::new("examples/hello_world_jsonrpc.idl");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is not set");
    let axum_generated = Path::new(&out_dir).join("hello_world.rs");
    let jsonrpc_generated = Path::new(&out_dir).join("hello_world_jsonrpc.rs");

    println!("cargo:rerun-if-changed={}", axum_idl.display());
    println!("cargo:rerun-if-changed={}", jsonrpc_idl.display());
    println!("cargo:rerun-if-changed=build.rs");

    Builder::new()
        .lang("rust-axum")
        .compile(&[axum_idl])
        .expect("failed to compile idl for rust-axum example");

    Builder::new()
        .lang("rust-jsonrpc")
        .compile(&[jsonrpc_idl])
        .expect("failed to compile idl for rust-jsonrpc example");

    normalize_generated_file(&axum_generated);
    normalize_generated_file(&jsonrpc_generated);
}

fn normalize_generated_file(path: &Path) {
    let content = fs::read_to_string(path).expect("failed to read generated file");
    let normalized = content
        .lines()
        .map(|line| {
            if let Some(rest) = line.strip_prefix("//!") {
                format!("//{}", rest)
            } else if let Some(rest) = line.strip_prefix("#![") {
                format!("#[{}", rest)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, normalized).expect("failed to write normalized generated file");
}
