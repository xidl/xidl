//! Reproduces issue #458: concurrent builds sharing one output directory and
//! using `with_output_filename` must not fail on the shared default filename.
//!
//! This file holds a single test because it mutates the process `PATH`,
//! which is shared across all threads of one test binary.
#![cfg(all(unix, not(feature = "bundle")))]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::Barrier;

/// The stub writes the default artifact, then stalls before exiting so the
/// first invocation still sits in "generation" while the second one has
/// already renamed the shared default file away.
#[test]
fn concurrent_output_filename_renames_do_not_race() {
    let scratch = std::env::temp_dir().join(format!("xidl-build-race-{}", std::process::id()));
    let bin_dir = scratch.join("bin");
    let out_dir = scratch.join("out");
    let counter = scratch.join("invocations");
    fs::create_dir_all(&bin_dir).expect("failed to create stub bin dir");
    fs::create_dir_all(&out_dir).expect("failed to create out dir");

    let stub = bin_dir.join("xidlc");
    let stub_source = format!(
        "#!/bin/sh\n\
         out=\"$3\"\n\
         printf '{{}}' > \"$out/openapi_city.json\"\n\
         echo x >> '{counter}'\n\
         n=$(wc -l < '{counter}')\n\
         if [ \"$n\" = 1 ]; then sleep 0.5; else sleep 0.05; fi\n\
         exit 0\n",
        counter = counter.display(),
    );
    fs::write(&stub, stub_source).expect("failed to write xidlc stub");
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755))
        .expect("failed to mark xidlc stub executable");

    let input = scratch.join("city.idl");
    fs::write(&input, "interface City { void noop(); };").expect("failed to write input idl");

    let previous_path = std::env::var_os("PATH");
    let mut search_paths = vec![bin_dir.clone()];
    search_paths.extend(std::env::split_paths(
        previous_path.as_deref().unwrap_or_default(),
    ));
    let test_path = std::env::join_paths(search_paths).expect("test executable search path");
    // SAFETY: single-test binary; see the module docs.
    unsafe {
        std::env::set_var("PATH", &test_path);
    }

    let barrier = std::sync::Arc::new(Barrier::new(2));
    let input_for_threads = input.clone();
    let out_dir_for_threads = out_dir.clone();
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let barrier = barrier.clone();
            let input = input_for_threads.clone();
            let out_dir = out_dir_for_threads.clone();
            std::thread::spawn(move || {
                barrier.wait();
                xidl_build::Builder::new()
                    .with_lang("openapi")
                    .with_out_dir(&out_dir)
                    .with_output_filename("city.json")
                    .compile(&[&input])
            })
        })
        .collect();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("compile thread panicked"))
        .collect();

    // SAFETY: single-test binary; see the module docs.
    unsafe {
        match previous_path {
            Some(path) => std::env::set_var("PATH", path),
            None => std::env::remove_var("PATH"),
        }
    }

    for result in results {
        result.expect("concurrent compile must succeed");
        let document = fs::read_to_string(out_dir.join("city.json"))
            .expect("renamed artifact must exist after a successful compile");
        assert!(
            document.contains("\"paths\"") || document == "{}",
            "{document}"
        );
    }
    assert!(
        !out_dir.join("openapi_city.json").exists(),
        "the default filename must not remain in the shared output directory"
    );

    let _ = fs::remove_dir_all(&scratch);
}
