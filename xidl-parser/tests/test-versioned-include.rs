use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use xidl_parser::hir;

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "xidl-parser-versioned-include-{name}-{}-{nanos}",
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

fn parse_hir(path: &Path) -> xidl_parser::error::ParserResult<hir::Specification> {
    let source = fs::read_to_string(path).expect("read fixture");
    let typed = xidl_parser::parser::parser_text_with_resolver(
        &source,
        Some(path.to_str().unwrap()),
        &mut hir::FsIncludeResolver,
    )?;
    hir::Specification::from_typed_ast_with_path(typed, path)
}

/// Module idents of `spec`, in declaration order.
fn module_idents(spec: &hir::Specification) -> Vec<String> {
    spec.0
        .iter()
        .filter_map(|definition| match definition {
            hir::Definition::ModuleDcl(module) => Some(module.ident.clone()),
            _ => None,
        })
        .collect()
}

/// Whether the module `ident` declares a member named `name`.
fn declares(spec: &hir::Specification, ident: &str, name: &str) -> bool {
    spec.0.iter().any(|definition| match definition {
        hir::Definition::ModuleDcl(module) if module.ident == ident => {
            module.definition.iter().any(|inner| {
                struct_names(std::slice::from_ref(inner))
                    .iter()
                    .any(|it| it == name)
            })
        }
        _ => false,
    })
}

const VERSIONED: &str = r#"
@version
enum ApiVersions {
    @rename("1.0") v1,
    @rename("2.0") v2,
};

struct Instant {
    string at;
    @added(ApiVersions::v2)
    @optional string monotonic;
};
"#;

#[test]
fn a_versioned_include_splices_only_that_version() {
    let dir = unique_temp_dir("splice");
    write_file(&dir.join("types.idl"), VERSIONED);
    let app = dir.join("app.idl");
    write_file(
        &app,
        "#include \"types.idl@v1\"\n\nstruct Event {\n    v1::Instant at;\n};\n",
    );

    let spec = parse_hir(&app).expect("versioned include parses");
    assert_eq!(module_idents(&spec), vec!["v1"]);
    assert!(declares(&spec, "v1", "Instant"));
    // The v2 member of `Instant` never reaches the including unit.
    let rendered = serde_json::to_string(&spec).expect("serialize");
    assert!(!rendered.contains("monotonic"), "{rendered}");
}

#[test]
fn a_version_a_file_does_not_declare_is_rejected() {
    let dir = unique_temp_dir("unknown-version");
    write_file(&dir.join("types.idl"), VERSIONED);
    let app = dir.join("app.idl");
    write_file(&app, "#include \"types.idl@v9\"\n");

    let err = parse_hir(&app).expect_err("unknown version is rejected");
    let message = err.to_string();
    assert!(message.contains("declares no version `v9`"), "{message}");
    assert!(message.contains("v1 (1.0)"), "{message}");
}

#[test]
fn a_version_on_an_unversioned_file_is_rejected() {
    let dir = unique_temp_dir("no-timeline");
    write_file(&dir.join("plain.idl"), "struct Plain { string value; };\n");
    let app = dir.join("app.idl");
    write_file(&app, "#include \"plain.idl@v1\"\n");

    let err = parse_hir(&app).expect_err("missing timeline is rejected");
    let message = err.to_string();
    assert!(
        message.contains("declares no version timeline"),
        "{message}"
    );
}

#[test]
fn a_plain_include_still_copies_the_whole_file() {
    let dir = unique_temp_dir("plain");
    write_file(&dir.join("types.idl"), VERSIONED);
    let app = dir.join("app.idl");
    write_file(
        &app,
        "#include \"types.idl\"\n\nstruct Event { string at; };\n",
    );

    let spec = parse_hir(&app).expect("plain include parses");
    assert_eq!(module_idents(&spec), Vec::<String>::new());
    assert!(struct_names(&spec.0).iter().any(|name| name == "Instant"));
}

/// Names of the structs `definitions` declare, modules included.
fn struct_names(definitions: &[hir::Definition]) -> Vec<String> {
    let mut names = Vec::new();
    for definition in definitions {
        match definition {
            hir::Definition::ModuleDcl(module) => {
                names.extend(struct_names(&module.definition));
            }
            hir::Definition::TypeDcl(hir::TypeDcl::ConstrTypeDcl(
                hir::ConstrTypeDcl::StructDcl(value),
            ))
            | hir::Definition::ConstrTypeDcl(hir::ConstrTypeDcl::StructDcl(value)) => {
                names.push(value.ident.clone());
            }
            _ => {}
        }
    }
    names
}
