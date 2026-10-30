//! Consumer contracts for isolated generation and renamed document publication.
#![cfg(feature = "bundle")]

use std::{fs, path::Path, sync::Barrier};

use serde_json::Value;
use xidl_build::{Builder, Error};

fn document(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).expect("published document"))
        .expect("complete JSON document")
}

fn assert_references(value: &Value, path: &Path) {
    match value {
        Value::Object(object) => {
            if let Some(Value::String(reference)) = object.get("$ref") {
                let (location, fragment) =
                    reference.split_once('#').expect("local schema reference");
                let target = if location.is_empty() {
                    path.to_path_buf()
                } else {
                    path.parent().expect("document directory").join(location)
                };
                assert!(
                    document(&target).pointer(fragment).is_some(),
                    "unresolved schema {reference} from {}",
                    path.display()
                );
            }
            for nested in object.values() {
                assert_references(nested, path);
            }
        }
        Value::Array(array) => {
            for nested in array {
                assert_references(nested, path);
            }
        }
        _ => {}
    }
}

#[test]
fn concurrent_requests_keep_complete_and_correct_documents() {
    let root = tempfile::tempdir().expect("consumer fixture");
    for language in ["openapi", "openrpc"] {
        let out = root.path().join(language);
        let mut inputs = Vec::new();
        for index in 0..8 {
            let dir = root.path().join(format!("contract-{index}"));
            fs::create_dir_all(&dir).expect("input directory");
            let input = dir.join("api.idl");
            fs::write(
                &input,
                format!(
                    "struct Receipt{index} {{ string message; }};\n\
                     interface Api {{ @get(path=\"/item\") Receipt{index} read(); }};"
                ),
            )
            .expect("input contract");
            inputs.push(input);
        }
        let barrier = Barrier::new(inputs.len());
        std::thread::scope(|scope| {
            let handles: Vec<_> = inputs
                .iter()
                .enumerate()
                .map(|(index, input)| {
                    let (barrier, out) = (&barrier, &out);
                    scope.spawn(move || {
                        barrier.wait();
                        let name = if index % 2 == 0 {
                            "shared.json".to_string()
                        } else {
                            format!("request-{index}.json")
                        };
                        Builder::new()
                            .with_lang(language)
                            .with_out_dir(out)
                            .with_output_filename(name)
                            .compile(&[input])
                    })
                })
                .collect();
            for handle in handles {
                handle
                    .join()
                    .expect("generation thread")
                    .expect("concurrent generation");
            }
        });
        for index in [1, 3, 5, 7] {
            let result = document(&out.join(format!("request-{index}.json")));
            assert!(
                result["components"]["schemas"]
                    .get(format!("Receipt{index}"))
                    .is_some()
            );
        }
        let shared = document(&out.join("shared.json"));
        assert!([0, 2, 4, 6].into_iter().any(|index| {
            shared["components"]["schemas"]
                .get(format!("Receipt{index}"))
                .is_some()
        }));
        assert!(fs::read_dir(&out).expect("output directory").all(|entry| {
            !entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .starts_with(".xidl-build-")
        }));
    }
}

#[test]
fn renamed_documents_keep_their_imports_and_reject_name_collisions() {
    let root = tempfile::tempdir().expect("import fixture");
    fs::create_dir(root.path().join("timestamp")).expect("import directory");
    fs::write(
        root.path().join("timestamp/index.idl"),
        "@version enum Versions { v1, v2 }; struct Instant { string at; };",
    )
    .expect("dependency contract");
    let input = root.path().join("app.idl");
    fs::write(
        &input,
        "import \"timestamp\"; interface Clock { @get(path=\"/now\") timestamp::v1::Instant now(); };",
    )
    .expect("entry contract");
    for language in ["openapi", "openrpc"] {
        let out = root.path().join(language);
        for name in [
            Path::new("nested/api.json").to_path_buf(),
            root.path().join(format!("absolute-{language}/api.json")),
            Path::new(&format!("{language}_app.json")).to_path_buf(),
        ] {
            Builder::new()
                .with_lang(language)
                .with_out_dir(&out)
                .with_output_filename(&name)
                .compile(&[&input])
                .expect("publish entry and dependencies");
            let entry = out.join(name);
            assert_references(&document(&entry), &entry);
            let dependency = entry
                .parent()
                .expect("entry directory")
                .join(format!("{language}_timestamp.v1.json"));
            assert_references(&document(&dependency), &dependency);
        }
        let collision = out.join(format!("{language}_timestamp.v1.json"));
        let previous = fs::read(&collision).expect("previous dependency");
        let failure = Builder::new()
            .with_lang(language)
            .with_out_dir(&out)
            .with_output_filename(&collision)
            .compile(&[&input])
            .expect_err("entry must not overwrite its own dependency");
        assert!(
            matches!(failure, Error::Config(message) if message.contains("supporting document"))
        );
        assert_eq!(fs::read(collision).expect("preserved dependency"), previous);
    }
}

#[test]
fn failed_generation_keeps_the_previous_artifact() {
    let root = tempfile::tempdir().expect("failure fixture");
    let output = root.path().join("api.json");
    fs::write(&output, "previous complete artifact").expect("previous artifact");
    let input = root.path().join("broken.idl");
    fs::write(&input, "interface Broken {{").expect("invalid contract");
    Builder::new()
        .with_lang("openapi")
        .with_out_dir(root.path())
        .with_output_filename("api.json")
        .compile(&[input])
        .expect_err("invalid contract must fail");
    assert_eq!(
        fs::read_to_string(output).expect("preserved artifact"),
        "previous complete artifact"
    );
    assert_eq!(
        fs::read_dir(root.path()).expect("fixture entries").count(),
        2
    );
}
