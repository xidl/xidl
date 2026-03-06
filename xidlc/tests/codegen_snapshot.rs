use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

fn lang_and_codegen(folder: &str) -> Option<&'static str> {
    match folder {
        "c" => Some("c"),
        "cpp" => Some("cpp"),
        "rust" => Some("rs"),
        "ts" => Some("ts"),
        "axum" => Some("axum"),
        "openapi" => Some("openapi"),
        _ => None,
    }
}

fn collect_idl_cases(root: &Path) -> Vec<(String, PathBuf)> {
    let mut cases = Vec::new();
    let entries = fs::read_dir(root).expect("read xidlc/tests");
    for entry in entries {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let lang = entry.file_name().to_string_lossy().to_string();
        if lang_and_codegen(&lang).is_none() {
            continue;
        }
        let files = fs::read_dir(&path).expect("read lang folder");
        for file in files {
            let file = file.expect("idl file");
            let case_path = file.path();
            if case_path.extension().and_then(|ext| ext.to_str()) != Some("idl") {
                continue;
            }
            cases.push((lang.clone(), case_path));
        }
    }
    cases.sort_by(|a, b| a.1.cmp(&b.1));
    cases
}

fn render_output(files: Vec<xidlc::driver::File>) -> String {
    let mut files = files;
    files.sort_by(|a, b| a.path().cmp(b.path()));

    let mut out = String::new();
    for file in files {
        out.push_str("===============\n");
        out.push_str(file.path());
        out.push_str("\n===============\n");
        out.push_str(file.content());
        if !file.content().ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

fn case_props(folder: &str, case_name: &str) -> HashMap<String, serde_json::Value> {
    let mut props: HashMap<String, serde_json::Value> =
        HashMap::from([(String::from("enable_metadata"), false.into())]);
    if folder == "rust" && matches!(case_name, "simple_union" | "option" | "enum_serialize") {
        props.insert("enable_serialize".to_string(), false.into());
        props.insert("enable_deserialize".to_string(), false.into());
        props.insert("enable_render_header".to_string(), false.into());
    }
    props
}

#[tokio::test(flavor = "current_thread")]
async fn codegen_snapshots_from_idl_folders() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let cases = collect_idl_cases(&root);
    assert!(!cases.is_empty(), "no idl cases found under xidlc/tests/*");

    for (folder, case_path) in cases {
        let lang = lang_and_codegen(&folder).expect("supported folder");
        let source = fs::read_to_string(&case_path).expect("read idl");
        let case_name = case_path
            .file_stem()
            .and_then(|value| value.to_str())
            .expect("case stem");
        let props = case_props(&folder, case_name);
        let mut generator = xidlc::driver::Generator::new(lang.to_string());
        let files = generator
            .generate_from_idl(
                &source,
                case_path
                    .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                    .unwrap_or(&case_path),
                props,
            )
            .await
            .expect("generate");
        let output = render_output(files);
        let snapshot_name = format!("{folder}__{case_name}");
        insta::assert_snapshot!(snapshot_name, output);
    }
}
