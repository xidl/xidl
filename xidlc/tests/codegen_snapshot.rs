use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

fn lang_and_codegen(folder: &str) -> Option<&'static str> {
    match folder {
        "rust" => Some("rs"),
        "ts" => Some("ts"),
        "golang" => Some("go"),
        "golang-http" => Some("go-http"),
        "python" => Some("python"),
        "python-http" => Some("python-http"),
        "axum" => Some("axum"),
        "openapi" => Some("openapi"),
        "openrpc" => Some("openrpc"),
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

fn render_single_output(path: &str, content: &str) -> String {
    let mut out = String::new();
    out.push_str("===============\n");
    out.push_str(path);
    out.push_str("\n===============\n");
    out.push_str(content);
    if !content.ends_with('\n') {
        out.push('\n');
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

async fn generate_case_output(
    folder: &str,
    case_name: &str,
    props: HashMap<String, serde_json::Value>,
) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let case_path = root.join(folder).join(format!("{case_name}.idl"));
    let lang = lang_and_codegen(folder).expect("supported folder");
    let source = fs::read_to_string(&case_path).expect("read idl");
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
    render_output(files)
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

#[cfg(feature = "cli")]
#[tokio::test(flavor = "current_thread")]
async fn skip_cdr_codec_matches_disabling_serialize_and_deserialize() {
    use clap::Parser;

    let args = xidlc::driver::ArgsGenerate {
        lang: "rust".to_string(),
        out_dir: ".".to_string(),
        client: false,
        server: true,
        dry_run: false,
        files: Vec::new(),
    };
    let mut expected_props = args.generator_props();
    expected_props.insert(String::from("enable_serialize"), false.into());
    expected_props.insert(String::from("enable_deserialize"), false.into());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let case_path = root.join("rust").join("struct_simple.idl");
    let out_dir = std::env::temp_dir().join(format!(
        "xidlc-skip-cdr-codec-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("unix epoch")
            .as_nanos()
    ));
    fs::create_dir_all(&out_dir).expect("create output dir");
    let cli = xidlc::cli::Cli::parse_from([
        "xidlc",
        "gen",
        "--out-dir",
        out_dir.to_str().expect("utf8 path"),
        "--skip-cdr-codec",
        "rust",
        case_path.to_str().expect("utf8 path"),
    ]);
    cli.run().await.expect("run cli");
    let actual = fs::read_to_string(out_dir.join("struct_simple.rs")).expect("read generated file");
    let expected = generate_case_output("rust", "struct_simple", expected_props).await;
    assert_eq!(render_single_output("struct_simple.rs", &actual), expected);
    fs::remove_dir_all(out_dir).expect("cleanup output dir");
}
