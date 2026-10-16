use std::collections::HashMap;
use std::fs;
use std::path::Path;

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

fn generate_adapters(props: HashMap<String, serde_json::Value>) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("ts-http")
        .join("versioned_adapters.idl");
    let source = fs::read_to_string(&path).expect("read idl");
    let mut props = props;
    props.insert(String::from("enable_client"), true.into());
    props.insert(String::from("enable_server"), true.into());
    let mut generator = xidlc::driver::Generator::new(String::from("typescript-rest"));
    let files = generator
        .generate_from_idl(&source, &path, props)
        .expect("generate versioned adapters");
    render_output(files)
}

fn generate_rust_adapters(props: HashMap<String, serde_json::Value>) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("axum")
        .join("versioned_adapters.idl");
    let source = fs::read_to_string(&path).expect("read idl");
    let mut props = props;
    props.insert(String::from("enable_client"), true.into());
    props.insert(String::from("enable_server"), true.into());
    let mut generator = xidlc::driver::Generator::new(String::from("rust-axum"));
    let files = generator
        .generate_from_idl(&source, &path, props)
        .expect("generate versioned rust adapters");
    render_output(files)
}

#[test]
fn rust_version_adapters_render_snapshot() {
    let output = generate_rust_adapters(HashMap::from([(
        String::from("enable_version_adapters"),
        true.into(),
    )]));
    insta::assert_snapshot!("versioned_adapters_rust_axum", output);
}

#[test]
fn rust_version_adapters_stay_off_by_default() {
    let output = generate_rust_adapters(HashMap::new());
    assert!(
        !output.contains("version_adapters"),
        "rust adapters must stay off unless enabled:\n{output}"
    );
}

#[test]
fn a_rust_version_may_not_expose_an_operation_the_head_removed() {
    let source = r#"
#pragma xidlc package retired_op

@version
enum ApiVersions {
    @rename("1.0") v1,
    @rename("2.0") v2,
};

interface Pets {
    @get(path = "/pets")
    string list();

    @removed(ApiVersions::v2)
    @get(path = "/legacy")
    string legacy();
};
"#;
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("axum")
        .join("versioned_adapters.idl");
    let mut generator = xidlc::driver::Generator::new(String::from("rust-axum"));
    let result = generator.generate_from_idl(
        source,
        &path,
        HashMap::from([
            (String::from("enable_version_adapters"), true.into()),
            (String::from("enable_client"), true.into()),
        ]),
    );
    let error = match result {
        Ok(_) => panic!("retired operations must fail rust generation"),
        Err(error) => error,
    };
    let message = error.to_string();
    assert!(message.contains("legacy"), "{message}");
    assert!(
        message.contains("cannot serve removed operations"),
        "{message}"
    );
}

#[test]
fn version_adapters_render_snapshot() {
    let output = generate_adapters(HashMap::from([(
        String::from("enable_version_adapters"),
        true.into(),
    )]));
    insta::assert_snapshot!("versioned_adapters_ts_http", output);
}

#[test]
fn version_adapters_stay_off_by_default() {
    let output = generate_adapters(HashMap::new());
    assert!(
        !output.contains("VersionAdapters"),
        "adapters must stay off unless enabled:\n{output}"
    );
}

#[test]
fn a_version_may_not_expose_an_operation_the_head_removed() {
    let source = r#"
#pragma xidlc package retired_op

@version
enum ApiVersions {
    @rename("1.0") v1,
    @rename("2.0") v2,
};

interface Pets {
    @get(path = "/pets")
    string list();

    @removed(ApiVersions::v2)
    @get(path = "/legacy")
    string legacy();
};
"#;
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("ts-http")
        .join("versioned_adapters.idl");
    let mut generator = xidlc::driver::Generator::new(String::from("typescript-rest"));
    let result = generator.generate_from_idl(
        source,
        &path,
        HashMap::from([
            (String::from("enable_version_adapters"), true.into()),
            (String::from("enable_client"), true.into()),
        ]),
    );
    let error = match result {
        Ok(_) => panic!("retired operations must fail generation"),
        Err(error) => error,
    };
    let message = error.to_string();
    assert!(message.contains("legacy"), "{message}");
    assert!(
        message.contains("phase 1 cannot serve removed operations"),
        "{message}"
    );
}
