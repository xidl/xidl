#![cfg(all(
    feature = "gen-rust-axum",
    feature = "gen-go-rest",
    feature = "gen-typescript-rest"
))]

use std::{collections::HashMap, path::Path};

const VERSIONED: &str = include_str!("../../bdd/features/data/stream_versions.idl");

fn render(target: &str, source: &str, path: &Path) -> String {
    let properties = HashMap::from([
        ("enable_client".into(), true.into()),
        ("enable_server".into(), true.into()),
    ]);
    let files = xidlc::driver::Generator::new(target.into())
        .generate_from_idl(source, path, properties)
        .unwrap();
    files
        .into_iter()
        .map(|file| format!("=== {} ===\n{}\n", file.path(), file.content()))
        .collect()
}

#[test]
fn stream_raises_use_only_the_selected_versions_exception_set() {
    for target in ["rust-axum", "typescript-rest", "go-rest"] {
        let output = render(target, VERSIONED, Path::new("stream_versions.idl"));
        assert!(!output.contains("V1V1Conflict"));
        assert!(!output.contains("V2V2Conflict"));
        match target {
            "go-rest" => {
                assert!(output.contains("*V1BillingConflict"));
                assert!(output.contains("*V2BillingRetired"));
                assert!(!output.contains("V1BillingRetired"));
            }
            "rust-axum" => assert!(output.contains("StreamException for")),
            _ => assert!(output.contains("failure.status, failure.body")),
        }
        insta::assert_snapshot!(format!("versioned_stream_{target}"), output);
    }
}

#[test]
fn included_business_contract_renders_all_stream_exception_paths() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../bdd/features/data/stream_lifecycle.idl");
    let source = std::fs::read_to_string(&path).unwrap();
    for target in ["rust-axum", "typescript-rest", "go-rest"] {
        let output = render(target, &source, &path);
        insta::assert_snapshot!(format!("ledger_stream_{target}"), output);
    }
}

#[test]
fn ordinary_typescript_target_uses_the_shared_terminal_aware_reader() {
    use xidlc::generate::typescript::{TsMode, TypescriptRender, TypescriptRenderer};
    let ast = xidl_parser::parser::parser_text("interface Api { @server_stream int32 ticks(); };")
        .unwrap();
    let spec = xidl_parser::hir::Specification::from(ast);
    let mut renderer = TypescriptRenderer::new().unwrap();
    renderer.extend(&HashMap::new());
    let output = spec
        .render("ticks", &renderer, TsMode::InterfaceOnly)
        .unwrap();
    assert!(output.client.contains("sseJsonStream"));
    assert!(
        !output.client.contains("sseJsonStreamLegacy")
            && !output.client.contains("ndjsonBodyLegacy")
    );
}
