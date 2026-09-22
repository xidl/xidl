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

fn generate_go_rest_with_props(props: HashMap<String, serde_json::Value>) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golang-http")
        .join("http_defaults.idl");
    let source = fs::read_to_string(&path).expect("read idl");
    let mut generator = xidlc::driver::Generator::new(String::from("go-rest"));
    let files = generator
        .generate_from_idl(&source, &path, props)
        .expect("generate go-rest");
    render_output(files)
}

fn generate_typescript_rest_with_props(props: HashMap<String, serde_json::Value>) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("ts-http")
        .join("http_defaults.idl");
    let source = fs::read_to_string(&path).expect("read idl");
    let mut generator = xidlc::driver::Generator::new(String::from("typescript-rest"));
    let files = generator
        .generate_from_idl(&source, &path, props)
        .expect("generate typescript-rest");
    render_output(files)
}

fn generate_go_rest_source(source: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golang-http")
        .join("inline.idl");
    let mut generator = xidlc::driver::Generator::new(String::from("go-rest"));
    let files = generator
        .generate_from_idl(source, &path, case_props("golang-http", "inline"))
        .expect("generate go-rest");
    render_output(files)
}

#[test]
fn codegen_snapshots_from_idl_folders() {
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
            .expect("generate");
        let output = render_output(files);
        let snapshot_name = format!("{folder}__{case_name}");
        insta::assert_snapshot!(snapshot_name, output);
    }
}

#[test]
fn go_rest_content_type_check_requires_explicit_consumes() {
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
    @request("json")
    string explicit_create(
        CreatePayload req
    );
};
"#,
    );

    assert_eq!(output.matches("GinRequireContentType").count(), 1);
    assert!(output.contains(r#"GinRequireContentType(c, "application/json")"#));
}

#[test]
fn generated_header_uses_compiler_metadata_overrides() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("shared")
        .join("basic.idl");
    let source = fs::read_to_string(&path).expect("read idl");
    let mut generator = xidlc::driver::Generator::new(String::from("rust"));
    let files = generator
        .generate_from_idl(&source, &path, case_props("rust", "basic"))
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

#[test]
fn go_rest_server_flag_omits_client_code() {
    let output = generate_go_rest_with_props(HashMap::from([
        (String::from("enable_client"), false.into()),
        (String::from("enable_server"), true.into()),
    ]));

    assert!(output.contains("type HttpDefaultsServiceService interface"));
    assert!(output.contains("func RegisterHttpDefaultsServiceHandler"));
    assert!(!output.contains("type HttpDefaultsServiceClient struct"));
    assert!(!output.contains("func NewHttpDefaultsServiceClient"));
}

#[test]
fn go_rest_client_flag_omits_server_code() {
    let output = generate_go_rest_with_props(HashMap::from([
        (String::from("enable_client"), true.into()),
        (String::from("enable_server"), false.into()),
    ]));

    assert!(output.contains("type HttpDefaultsServiceClient struct"));
    assert!(output.contains("func NewHttpDefaultsServiceClient"));
    assert!(!output.contains("type HttpDefaultsServiceService interface"));
    assert!(!output.contains("func RegisterHttpDefaultsServiceHandler"));
    assert!(!output.contains("\"github.com/gin-gonic/gin\""));
}

#[test]
fn go_rest_rejects_empty_client_server_mode() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golang-http")
        .join("http_defaults.idl");
    let source = fs::read_to_string(&path).expect("read idl");
    let mut generator = xidlc::driver::Generator::new(String::from("go-rest"));
    let result = generator.generate_from_idl(
        &source,
        &path,
        HashMap::from([
            (String::from("enable_client"), false.into()),
            (String::from("enable_server"), false.into()),
        ]),
    );
    let err = match result {
        Ok(_) => panic!("empty go-rest mode should fail"),
        Err(err) => err,
    };

    assert!(err.to_string().contains("enable_client or enable_server"));
}

#[test]
fn typescript_rest_server_flag_omits_client_code() {
    let output = generate_typescript_rest_with_props(HashMap::from([
        (String::from("enable_client"), false.into()),
        (String::from("enable_server"), true.into()),
    ]));

    assert!(output.contains("http_defaults.server.ts"));
    assert!(output.contains("HttpDefaultsServiceOperations"));
    assert!(!output.contains("http_defaults.client.ts"));
}

#[test]
fn typescript_rest_client_flag_omits_server_code() {
    let output = generate_typescript_rest_with_props(HashMap::from([
        (String::from("enable_client"), true.into()),
        (String::from("enable_server"), false.into()),
    ]));

    assert!(output.contains("http_defaults.client.ts"));
    assert!(!output.contains("http_defaults.server.ts"));
    assert!(!output.contains("HttpDefaultsServiceOperations"));
}

#[test]
fn typescript_rest_defaults_to_client_only() {
    let output = generate_typescript_rest_with_props(HashMap::new());

    assert!(output.contains("http_defaults.client.ts"));
    assert!(!output.contains("http_defaults.server.ts"));
    assert!(!output.contains("HttpDefaultsServiceOperations"));
}

#[test]
fn typescript_rest_rejects_empty_client_server_mode() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("ts-http")
        .join("http_defaults.idl");
    let source = fs::read_to_string(&path).expect("read idl");
    let mut generator = xidlc::driver::Generator::new(String::from("typescript-rest"));
    let result = generator.generate_from_idl(
        &source,
        &path,
        HashMap::from([
            (String::from("enable_client"), false.into()),
            (String::from("enable_server"), false.into()),
        ]),
    );
    let err = match result {
        Ok(_) => panic!("empty typescript-rest mode should fail"),
        Err(err) => err,
    };

    assert!(err.to_string().contains("enable_client or enable_server"));
}

fn generate_typescript_rest_source(source: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("ts-http")
        .join("inline.idl");
    let mut generator = xidlc::driver::Generator::new(String::from("typescript-rest"));
    let files = generator
        .generate_from_idl(
            source,
            &path,
            HashMap::from([
                (String::from("enable_client"), true.into()),
                (String::from("enable_server"), true.into()),
            ]),
        )
        .expect("generate typescript-rest");
    render_output(files)
}

#[test]
fn typescript_rest_reserved_word_query_param_keys_match_schema() {
    let output = generate_typescript_rest_source(
        r#"
#pragma xidlc package xidlc

interface PanelApi {
    @get(path = "/api/v1/usage/hour")
    any getUsageHour(
        @query string userId,
        @query string from,
        @query string to
    );
};
"#,
    );

    // Server binding keys must use the schema field name, not the TS-escaped ident.
    assert!(
        output.contains("{ wireName: \"from\", key: \"from\", multi: false }"),
        "server query binding should keep schema key from:\n{output}"
    );
    assert!(
        !output.contains("key: \"_from\""),
        "server binding must not escape reserved-word keys:\n{output}"
    );

    // args are looked up on the schema-parsed payload.
    assert!(
        output.contains("args: [\"userId\", \"from\", \"to\"]"),
        "server args should use schema field names:\n{output}"
    );

    // Zod request schema field stays the raw IDL name.
    assert!(
        output.contains("\"from\": z.coerce.string()")
            || output.contains("from: z.coerce.string()"),
        "request schema should expose field from:\n{output}"
    );

    // Handler parameter remains TS-escaped so generated code compiles.
    assert!(
        output.contains("userId: string, _from: string, to: string"),
        "handler signature should escape reserved-word params:\n{output}"
    );
}

#[test]
fn typescript_rest_reserved_word_body_param_keys_match_schema() {
    let output = generate_typescript_rest_source(
        r#"
#pragma xidlc package xidlc

struct CreatePayload {
    string name;
};

interface PanelApi {
    @post(path = "/api/v1/usage")
    any createUsage(
        @query string from,
        CreatePayload req
    );
};
"#,
    );

    assert!(
        output.contains("{ wireName: \"from\", key: \"from\", multi: false }"),
        "server query binding should keep schema key from:\n{output}"
    );
    assert!(
        !output.contains("key: \"_from\"") && !output.contains("key: \"_req\""),
        "server bindings must not escape reserved-word keys:\n{output}"
    );
    assert!(
        output.contains("args: [\"from\", \"req\"]"),
        "server args should use schema field names:\n{output}"
    );
}

#[test]
fn typescript_rest_reserved_word_response_keys_match_schema() {
    let output = generate_typescript_rest_source(
        r#"
#pragma xidlc package xidlc

interface PanelApi {
    @get(path = "/api/v1/usage")
    any getUsage(
        out string from,
        out string to
    );
};
"#,
    );

    assert!(
        !output.contains("key: \"_from\"") && !output.contains("key: \"\"from\""),
        "response binding keys must be the schema field name:\n{output}"
    );
    assert!(
        output.contains("key: \"from\""),
        "response binding should expose key from:\n{output}"
    );
}
