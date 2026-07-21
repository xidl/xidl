use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

fn lang_and_codegen(folder: &str) -> Option<&'static str> {
    match folder {
        "rust" => Some("rs"),
        "ts" => Some("ts"),
        "ts-http" => Some("typescript-rest"),
        "golang" => Some("go"),
        "golang-http" => Some("go-rest"),
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
            if lang == "ts"
                && case_path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.starts_with("http_"))
            {
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
    let _ = case_name;
    let mut props = HashMap::from([(String::from("enable_metadata"), true.into())]);
    if folder == "ts-http" {
        props.insert(String::from("enable_client"), true.into());
        props.insert(String::from("enable_server"), true.into());
    }
    props
}

async fn generate_go_rest_with_props(props: HashMap<String, serde_json::Value>) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golang-http")
        .join("http_defaults.idl");
    let source = fs::read_to_string(&path).expect("read idl");
    let mut generator = xidlc::driver::Generator::new(String::from("go-rest"));
    let files = generator
        .generate_from_idl(&source, &path, props)
        .await
        .expect("generate go-rest");
    render_output(files)
}

async fn generate_typescript_rest_with_props(props: HashMap<String, serde_json::Value>) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("ts-http")
        .join("http_defaults.idl");
    let source = fs::read_to_string(&path).expect("read idl");
    let mut generator = xidlc::driver::Generator::new(String::from("typescript-rest"));
    let files = generator
        .generate_from_idl(&source, &path, props)
        .await
        .expect("generate typescript-rest");
    render_output(files)
}

async fn generate_go_rest_source(source: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golang-http")
        .join("inline.idl");
    let mut generator = xidlc::driver::Generator::new(String::from("go-rest"));
    let files = generator
        .generate_from_idl(source, &path, case_props("golang-http", "inline"))
        .await
        .expect("generate go-rest");
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

#[tokio::test(flavor = "current_thread")]
async fn go_rest_content_type_check_requires_explicit_consumes() {
    let output = generate_go_rest_source(
        r#"
#pragma xidlc package xidlc

struct CreatePayload {
    string name;
};

interface ContentTypeCheckService {
    @post(path = "/implicit")
    string implicit_create(
        CreatePayload req
    );

    @post(path = "/explicit")
    @Consumes("application/json")
    string explicit_create(
        CreatePayload req
    );
};
"#,
    )
    .await;

    assert_eq!(output.matches("GinRequireContentType").count(), 1);
    assert!(output.contains(r#"GinRequireContentType(c, "application/json")"#));
}

#[tokio::test(flavor = "current_thread")]
async fn generated_header_uses_compiler_metadata_overrides() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("shared")
        .join("basic.idl");
    let source = fs::read_to_string(&path).expect("read idl");
    let mut generator = xidlc::driver::Generator::new(String::from("rust"));
    let files = generator
        .generate_from_idl(&source, &path, case_props("rust", "basic"))
        .await
        .expect("generate");
    let output = render_output(files);
    let expected = format!(
        "// Code generated by xidlc-v{}-{}. DO NOT EDIT.",
        option_env!("XIDLC_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")),
        option_env!("XIDLC_GIT_HASH").unwrap_or("unknown")
    );
    assert!(
        output.contains(&expected),
        "generated header should use compiler metadata overrides: {output}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn go_rest_server_flag_omits_client_code() {
    let output = generate_go_rest_with_props(HashMap::from([
        (String::from("enable_client"), false.into()),
        (String::from("enable_server"), true.into()),
    ]))
    .await;

    assert!(output.contains("type HttpDefaultsServiceService interface"));
    assert!(output.contains("func RegisterHttpDefaultsServiceHandler"));
    assert!(!output.contains("type HttpDefaultsServiceClient struct"));
    assert!(!output.contains("func NewHttpDefaultsServiceClient"));
}

#[tokio::test(flavor = "current_thread")]
async fn go_rest_client_flag_omits_server_code() {
    let output = generate_go_rest_with_props(HashMap::from([
        (String::from("enable_client"), true.into()),
        (String::from("enable_server"), false.into()),
    ]))
    .await;

    assert!(output.contains("type HttpDefaultsServiceClient struct"));
    assert!(output.contains("func NewHttpDefaultsServiceClient"));
    assert!(!output.contains("type HttpDefaultsServiceService interface"));
    assert!(!output.contains("func RegisterHttpDefaultsServiceHandler"));
    assert!(!output.contains("\"github.com/gin-gonic/gin\""));
}

#[tokio::test(flavor = "current_thread")]
async fn go_rest_rejects_empty_client_server_mode() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golang-http")
        .join("http_defaults.idl");
    let source = fs::read_to_string(&path).expect("read idl");
    let mut generator = xidlc::driver::Generator::new(String::from("go-rest"));
    let result = generator
        .generate_from_idl(
            &source,
            &path,
            HashMap::from([
                (String::from("enable_client"), false.into()),
                (String::from("enable_server"), false.into()),
            ]),
        )
        .await;
    let err = match result {
        Ok(_) => panic!("empty go-rest mode should fail"),
        Err(err) => err,
    };

    assert!(err.to_string().contains("enable_client or enable_server"));
}

#[tokio::test(flavor = "current_thread")]
async fn typescript_rest_server_flag_omits_client_code() {
    let output = generate_typescript_rest_with_props(HashMap::from([
        (String::from("enable_client"), false.into()),
        (String::from("enable_server"), true.into()),
    ]))
    .await;

    assert!(output.contains("http_defaults.server.ts"));
    assert!(output.contains("HttpDefaultsServiceOperations"));
    assert!(!output.contains("http_defaults.client.ts"));
}

#[tokio::test(flavor = "current_thread")]
async fn typescript_rest_client_flag_omits_server_code() {
    let output = generate_typescript_rest_with_props(HashMap::from([
        (String::from("enable_client"), true.into()),
        (String::from("enable_server"), false.into()),
    ]))
    .await;

    assert!(output.contains("http_defaults.client.ts"));
    assert!(!output.contains("http_defaults.server.ts"));
    assert!(!output.contains("HttpDefaultsServiceOperations"));
}

#[tokio::test(flavor = "current_thread")]
async fn typescript_rest_defaults_to_client_only() {
    let output = generate_typescript_rest_with_props(HashMap::new()).await;

    assert!(output.contains("http_defaults.client.ts"));
    assert!(!output.contains("http_defaults.server.ts"));
    assert!(!output.contains("HttpDefaultsServiceOperations"));
}

#[tokio::test(flavor = "current_thread")]
async fn typescript_rest_rejects_empty_client_server_mode() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("ts-http")
        .join("http_defaults.idl");
    let source = fs::read_to_string(&path).expect("read idl");
    let mut generator = xidlc::driver::Generator::new(String::from("typescript-rest"));
    let result = generator
        .generate_from_idl(
            &source,
            &path,
            HashMap::from([
                (String::from("enable_client"), false.into()),
                (String::from("enable_server"), false.into()),
            ]),
        )
        .await;
    let err = match result {
        Ok(_) => panic!("empty typescript-rest mode should fail"),
        Err(err) => err,
    };

    assert!(err.to_string().contains("enable_client or enable_server"));
}
