//! Issue #458 regression for the bundled backend: two concurrent compile
//! calls sharing one output directory and one `with_output_filename` name
//! must both succeed and leave only the requested artifact.
#![cfg(feature = "bundle")]

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};

const HELLO_WORLD_IDL: &str = include_str!("fixtures/hello_world.idl");

#[test]
fn bundle_backend_concurrent_output_filename_renames_do_not_race() {
    let scratch =
        std::env::temp_dir().join(format!("xidl-build-bundle-race-{}", std::process::id()));
    let out_dir = scratch.join("out");
    let input = scratch.join("hello_world.idl");
    fs::create_dir_all(&out_dir).expect("failed to create out dir");
    fs::write(&input, HELLO_WORLD_IDL).expect("failed to write input idl");

    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let barrier = barrier.clone();
            let input = input.clone();
            let out_dir = out_dir.clone();
            std::thread::spawn(move || {
                barrier.wait();
                xidl_build::Builder::new()
                    .with_lang("openapi")
                    .with_out_dir(&out_dir)
                    .with_output_filename("hello.json")
                    .compile(&[&input])
            })
        })
        .collect();

    for handle in handles {
        handle
            .join()
            .expect("compile thread panicked")
            .expect("concurrent bundled compile must succeed");
    }

    let document = fs::read_to_string(out_dir.join("hello.json")).expect("renamed document");
    assert!(document.contains("\"openapi\""), "{document}");
    assert!(
        !out_dir.join("openapi_hello_world.json").exists(),
        "the default filename must not remain in the shared output directory"
    );
    let staging_leftovers: Vec<PathBuf> = fs::read_dir(&out_dir)
        .expect("failed to list out dir")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.file_name().is_some_and(|name| name != "hello.json"))
        .collect();
    assert!(
        staging_leftovers.is_empty(),
        "staging directories must be cleaned up: {staging_leftovers:?}"
    );

    let _ = fs::remove_dir_all(&scratch);
}
