//! End-to-end test of the default CLI backend against a stub `xidlc`.
//!
//! The file holds a single test because it mutates the process `PATH`,
//! which is shared across all threads of one test binary.
#![cfg(all(unix, not(feature = "bundle")))]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use xidl_build::Error;

#[test]
fn cli_backend_invokes_xidlc_from_path() {
    let scratch = std::env::temp_dir().join(format!("xidl-build-cli-{}", std::process::id()));
    let bin_dir = scratch.join("bin");
    let out_dir = scratch.join("out");
    let argv_log = scratch.join("argv.log");
    fs::create_dir_all(&bin_dir).expect("failed to create stub bin dir");
    fs::create_dir_all(&out_dir).expect("failed to create stub out dir");

    let stub = bin_dir.join("xidlc");
    let stub_source = format!(
        "#!/bin/sh\n\
         echo \"$@\" > '{argv_log}'\n\
         out=\"$3\"\n\
         case \"$4\" in\n\
         openapi) printf '{{}}' > \"$out/openapi_city.json\" ;;\n\
         esac\n\
         exit 0\n",
        argv_log = argv_log.display(),
    );
    fs::write(&stub, stub_source).expect("failed to write xidlc stub");
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755))
        .expect("failed to mark xidlc stub executable");

    let input = scratch.join("city.idl");
    fs::write(&input, "interface City { void noop(); };").expect("failed to write input idl");

    let previous_path = std::env::var_os("PATH");
    // SAFETY: single-test binary; see the module docs.
    unsafe {
        std::env::set_var("PATH", &bin_dir);
    }

    xidl_build::Builder::new()
        .with_lang("openapi")
        .with_out_dir(&out_dir)
        .with_output_filename("city.json")
        .compile(&[&input])
        .expect("stubbed xidlc CLI should succeed");

    let argv = fs::read_to_string(&argv_log).expect("stub did not record argv");
    let argv = argv.trim();
    let expected_tail = format!(" openapi {}", input.display());
    assert!(argv.ends_with(&expected_tail), "unexpected argv: {argv}");
    let out_dir_arg = argv
        .strip_prefix("gen --out-dir ")
        .and_then(|rest| rest.strip_suffix(&expected_tail))
        .unwrap_or_else(|| panic!("unexpected argv: {argv}"));
    let staging = Path::new(out_dir_arg);
    assert!(
        staging.starts_with(&out_dir)
            && staging
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(".xidl-build-stage-")),
        "generation must target a staging directory inside out_dir, got {out_dir_arg}"
    );
    assert!(
        out_dir.join("city.json").exists(),
        "renamed artifact missing in {}",
        out_dir.display()
    );

    // Each subprocess waits until both generators have completed their writes.
    // With a shared intermediate, one of the two renames must then fail.
    for shared_destination in [true, false] {
        let ready = scratch.join(format!("ready-{shared_destination}"));
        fs::create_dir_all(&ready).expect("create subprocess barrier");
        let source = format!(
            r#"#!/bin/sh
out="$3"
printf '{{"openapi":"3.1.0","source":"%s"}}' "$5" > "$out/openapi_city.json"
: > '{ready}/'"$$"
attempt=0
while :; do
    set -- '{ready}/'*
    [ "$#" -ge 2 ] && break
    attempt=$((attempt + 1))
    [ "$attempt" -lt 500 ] || exit 9
    /bin/sleep 0.01
done
"#,
            ready = ready.display(),
        );
        fs::write(&stub, source).expect("write synchronized generator");
        let inputs: Vec<_> = ["left", "right"]
            .into_iter()
            .map(|side| {
                let dir = scratch.join(side);
                fs::create_dir_all(&dir).expect("create distinct input directory");
                let path = dir.join("city.idl");
                fs::write(&path, "interface City { void noop(); };").expect("write input");
                path
            })
            .collect();
        std::thread::scope(|scope| {
            let handles: Vec<_> = inputs
                .iter()
                .enumerate()
                .map(|(index, input)| {
                    let out_dir = &out_dir;
                    scope.spawn(move || {
                        let name = if shared_destination {
                            "shared.json".to_string()
                        } else {
                            format!("request-{index}.json")
                        };
                        xidl_build::Builder::new()
                            .with_lang("openapi")
                            .with_out_dir(out_dir)
                            .with_output_filename(name)
                            .compile(&[input])
                    })
                })
                .collect();
            for handle in handles {
                handle
                    .join()
                    .expect("generation thread")
                    .expect("both subprocesses must publish successfully");
            }
        });
        for index in 0..2 {
            let name = if shared_destination {
                "shared.json".to_string()
            } else {
                format!("request-{index}.json")
            };
            let bytes = fs::read(out_dir.join(name)).expect("published document");
            let document: serde_json::Value =
                serde_json::from_slice(&bytes).expect("complete JSON document");
            let source = document["source"].as_str().expect("document provenance");
            if shared_destination {
                assert!(inputs.iter().any(|input| input.to_str() == Some(source)));
            } else {
                assert_eq!(source, inputs[index].to_str().expect("input path"));
            }
        }
    }
    assert!(
        fs::read_dir(&out_dir)
            .expect("published directory")
            .all(|entry| !entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .starts_with(".xidl-build-")),
        "private generation directories must be cleaned"
    );

    fs::write(&stub, "#!/bin/sh\necho 'idl parse failure' >&2\nexit 3\n")
        .expect("failed to rewrite xidlc stub");
    let failure = xidl_build::Builder::new()
        .with_lang("openapi")
        .with_out_dir(&out_dir)
        .with_output_filename("city.json")
        .compile(&[&input])
        .expect_err("failing xidlc stub should surface an error");
    assert_eq!(
        fs::read_to_string(out_dir.join("city.json")).expect("previous output"),
        "{}"
    );
    match failure {
        Error::Compile(message) => {
            assert!(
                message.contains("idl parse failure"),
                "unexpected message: {message}"
            );
            assert!(
                message.contains("exit status: 3"),
                "unexpected message: {message}"
            );
        }
        other => panic!("expected Error::Compile, got {other:?}"),
    }

    fs::remove_file(&stub).expect("failed to remove xidlc stub");
    let missing = xidl_build::Builder::new()
        .with_lang("openapi")
        .with_out_dir(&out_dir)
        .compile(&[&input])
        .expect_err("missing xidlc CLI should surface an error");
    match missing {
        Error::Config(message) => {
            assert!(
                message.contains("not found on PATH"),
                "unexpected message: {message}"
            );
        }
        other => panic!("expected Error::Config, got {other:?}"),
    }

    // SAFETY: single-test binary; see the module docs.
    unsafe {
        match previous_path {
            Some(path) => std::env::set_var("PATH", path),
            None => std::env::remove_var("PATH"),
        }
    }
    let _ = fs::remove_dir_all(&scratch);
}
