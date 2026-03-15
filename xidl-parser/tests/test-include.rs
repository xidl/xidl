use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "xidl-parser-include-{name}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn write_file(path: &Path, source: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(path, source).expect("write fixture");
}

fn parse_hir(path: &Path) -> xidl_parser::error::ParserResult<xidl_parser::hir::Specification> {
    let source = fs::read_to_string(path).expect("read fixture");
    let typed = xidl_parser::parser::parser_text(&source)?;
    xidl_parser::hir::Specification::from_typed_ast_with_path(typed, path)
}

#[test]
fn test_hir_include_inserts_definitions_in_order() {
    let root = unique_temp_dir("ordered");
    let main = root.join("main.idl");
    let shared = root.join("shared.idl");

    write_file(
        &main,
        r#"
        struct Before {
            int32 value;
        };

        #include "shared.idl"

        struct After {
            int32 value;
        };
        "#,
    );
    write_file(
        &shared,
        r#"
        struct Shared {
            int32 value;
        };
        "#,
    );

    let hir = parse_hir(&main).unwrap();
    insta::assert_debug_snapshot!("hir__include_ordered", hir);
}

#[test]
fn test_hir_nested_include_preserves_depth_first_order() {
    let root = unique_temp_dir("nested");
    let main = root.join("main.idl");
    let middle = root.join("parts/middle.idl");
    let leaf = root.join("parts/leaf.idl");

    write_file(
        &main,
        r#"
        struct Start {
            int32 value;
        };

        #include "parts/middle.idl"

        struct End {
            int32 value;
        };
        "#,
    );
    write_file(
        &middle,
        r#"
        struct Middle {
            int32 value;
        };

        #include "leaf.idl"
        "#,
    );
    write_file(
        &leaf,
        r#"
        struct Leaf {
            int32 value;
        };
        "#,
    );

    let hir = parse_hir(&main).unwrap();
    insta::assert_debug_snapshot!("hir__include_nested", hir);
}

#[test]
fn test_hir_include_missing_file_errors() {
    let root = unique_temp_dir("missing");
    let main = root.join("main.idl");

    write_file(&main, "#include \"missing.idl\"\n");

    let err = parse_hir(&main).expect_err("missing include should fail");
    assert!(err.to_string().contains("does not exist"));
    assert!(err.to_string().contains("missing.idl"));
}

#[test]
fn test_hir_include_invalid_ast_errors() {
    let root = unique_temp_dir("invalid");
    let main = root.join("main.idl");
    let invalid = root.join("invalid.idl");

    write_file(&main, "#include \"invalid.idl\"\n");
    write_file(&invalid, "struct Broken { int32 value \n");

    let err = parse_hir(&main).expect_err("invalid include should fail");
    assert!(err.to_string().contains("failed to parse include"));
    assert!(err.to_string().contains("invalid.idl"));
}

#[test]
fn test_hir_include_cycle_errors() {
    let root = unique_temp_dir("cycle");
    let a = root.join("a.idl");
    let b = root.join("b.idl");

    write_file(&a, "#include \"b.idl\"\n");
    write_file(&b, "#include \"a.idl\"\n");

    let err = parse_hir(&a).expect_err("cyclic include should fail");
    assert!(err.to_string().contains("cyclic include detected"));
    assert!(err.to_string().contains("a.idl"));
    assert!(err.to_string().contains("b.idl"));
}
