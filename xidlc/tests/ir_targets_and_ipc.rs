use std::collections::HashMap;
use xidlc::generate::hir_gen::HirGen;
use xidlc::generate::typed_ast_gen::TypedAstGen;
use xidlc::jsonrpc::{Codegen, CodegenInput};

#[test]
fn jsonrpc_hir_and_rest_hir_aliases_generate_json_without_recursion() {
    let source = "interface Echo { string ping(in string msg); };";
    for lang in ["jsonrpc-hir", "jsonrpc_hir", "rest-hir", "rest_hir"] {
        let files = xidlc::generate_from_source(lang, source, HashMap::new())
            .unwrap_or_else(|err| panic!("target '{lang}' failed: {err}"));
        assert_eq!(files.len(), 1, "target '{lang}' should emit one file");
        let parsed: serde_json::Value =
            serde_json::from_str(files[0].content()).expect("IR output should be valid JSON");
        assert!(
            parsed.is_object(),
            "target '{lang}' should emit JSON object"
        );
    }
}

#[test]
fn hir_and_typed_ast_stages_return_typed_errors_when_required_props_missing() {
    let empty_input = CodegenInput::new_rpc_hir(xidl_parser::hir::Specification(vec![]));
    let hir_err = HirGen
        .generate(empty_input.clone(), "input.idl".into(), HashMap::new())
        .expect_err("HirGen should reject missing idl property");
    assert!(hir_err.message.contains("missing required property 'idl'"));

    let ast_err = TypedAstGen
        .generate(empty_input, "input.idl".into(), HashMap::new())
        .expect_err("TypedAstGen should reject missing idl property");
    assert!(ast_err.message.contains("missing required property 'idl'"));
}
