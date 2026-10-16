//! End-to-end tests of the bundled in-process compiler backend.
#![cfg(feature = "bundle")]

use std::fs;
use std::path::PathBuf;
use xidl_build::Error;

const HELLO_WORLD_IDL: &str = include_str!("fixtures/hello_world.idl");

fn scratch_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("xidl-build-bundle-{}-{label}", std::process::id()))
}

#[test]
fn bundle_backend_generates_openapi_artifact() {
    let scratch = scratch_dir("openapi");
    let out_dir = scratch.join("out");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let input = scratch.join("hello_world.idl");
    fs::write(&input, HELLO_WORLD_IDL).expect("failed to write input idl");

    xidl_build::Builder::new()
        .with_lang("openapi")
        .with_out_dir(&out_dir)
        .compile(&[&input])
        .expect("bundled openapi generation should succeed");

    assert!(
        out_dir.join("openapi_hello_world.json").exists(),
        "openapi artifact missing in {}",
        out_dir.display()
    );
    let _ = fs::remove_dir_all(&scratch);
}

#[test]
fn bundle_backend_accepts_a_library_directory() {
    let scratch = scratch_dir("library");
    let out_dir = scratch.join("out");
    let library = scratch.join("library");
    fs::create_dir_all(&library).expect("failed to create library dir");
    fs::write(
        library.join("xidl.toml"),
        "name = \"greeter\"\nversion = \"1.0.0\"\n",
    )
    .expect("failed to write manifest");
    fs::write(library.join("hello_world.idl"), HELLO_WORLD_IDL).expect("failed to write input idl");
    fs::write(library.join("notes.txt"), "not an input").expect("failed to write decoy");

    xidl_build::Builder::new()
        .with_lang("openapi")
        .with_out_dir(&out_dir)
        .compile(&[&library])
        .expect("a bundled library directory should compile");

    assert!(
        out_dir.join("openapi_hello_world.json").exists(),
        "the manifest inputs should reach the generator in {}",
        out_dir.display()
    );
    let _ = fs::remove_dir_all(&scratch);
}

#[test]
fn bundle_backend_reports_compile_diagnostics() {
    let scratch = scratch_dir("diagnostics");
    let out_dir = scratch.join("out");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let input = scratch.join("broken.idl");
    fs::write(&input, "interface Broken {{").expect("failed to write input idl");

    let failure = xidl_build::Builder::new()
        .with_lang("openapi")
        .with_out_dir(&out_dir)
        .compile(&[&input])
        .expect_err("malformed IDL must fail bundled generation");
    match failure {
        Error::Compile(message) => {
            assert!(
                message.contains("broken.idl"),
                "unexpected message: {message}"
            );
        }
        other => panic!("expected Error::Compile, got {other:?}"),
    }
    let _ = fs::remove_dir_all(&scratch);
}

const VERSIONED_IDL: &str = r#"
#pragma xidlc package hello_versioned

@version
enum ApiVersions {
    @rename("1.0") v1,
    @rename("2.0") v2,
};

struct Hello {
    string message;
    @added(ApiVersions::v2)
    @optional string note;
};

interface Greeter {
    @get(path = "/hello")
    Hello hello();
};
"#;

#[test]
fn bundle_backend_selects_and_renames_a_versioned_document() {
    let scratch = scratch_dir("versioned");
    let out_dir = scratch.join("out");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let input = scratch.join("versioned.idl");
    fs::write(&input, VERSIONED_IDL).expect("failed to write input idl");

    xidl_build::Builder::new()
        .with_lang("openapi")
        .with_api_version("v1")
        .with_output_filename("api.json")
        .with_out_dir(&out_dir)
        .compile(&[&input])
        .expect("versioned openapi generation should succeed");

    let document = fs::read_to_string(out_dir.join("api.json")).expect("renamed document");
    assert!(document.contains("\"version\": \"1.0\""), "{document}");

    let failure = xidl_build::Builder::new()
        .with_lang("openapi")
        .with_output_filename("api.json")
        .with_out_dir(&out_dir)
        .compile(&[&input])
        .expect_err("several versioned documents need an explicit selection");
    match failure {
        Error::Compile(message) => assert!(
            message.contains("one document per API version"),
            "unexpected message: {message}"
        ),
        other => panic!("expected Error::Compile, got {other:?}"),
    }
    let _ = fs::remove_dir_all(&scratch);
}
