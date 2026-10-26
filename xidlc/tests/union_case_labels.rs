use std::collections::HashMap;
use std::path::Path;

#[test]
fn every_target_rejects_bare_enum_labels_before_rendering() {
    let source = include_str!("../../bdd/features/data/union_unqualified_case.idl");
    let targets = [
        "hir",
        "rest-hir",
        "version-map",
        #[cfg(feature = "gen-rust")]
        "rust",
        #[cfg(feature = "gen-rust-axum")]
        "rust-axum",
        #[cfg(feature = "gen-rust-axum-adapters")]
        "rust-axum-adapters",
        #[cfg(feature = "gen-typescript")]
        "typescript",
        #[cfg(feature = "gen-typescript-rest")]
        "typescript-rest",
        #[cfg(feature = "gen-typescript-rest-adapters")]
        "typescript-rest-adapters",
        #[cfg(feature = "gen-go")]
        "go",
        #[cfg(feature = "gen-go-rest")]
        "go-rest",
        #[cfg(feature = "gen-openapi")]
        "openapi",
    ];
    for target in targets {
        let result = xidlc::driver::Generator::new(target.into()).generate_from_idl(
            source,
            Path::new("union_unqualified_case.idl"),
            HashMap::new(),
        );
        let error = match result {
            Ok(_) => panic!("{target} accepted a bare enum label"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("union 'Payload'"), "{target}: {error}");
        assert!(error.contains("case label 'mms'"), "{target}: {error}");
        assert!(error.contains("Enum::member"), "{target}: {error}");
    }
}

#[cfg(feature = "gen-rust")]
#[test]
fn documented_union_uses_qualified_labels_and_struct_payloads() {
    let guide = include_str!("../../website/src/content/docs/docs/index.mdx");
    let section = guide.split("## Union").nth(1).expect("union guide");
    let source = section
        .split("```idl")
        .nth(1)
        .expect("union IDL example")
        .split("```")
        .next()
        .expect("IDL fence");
    let files = xidlc::driver::Generator::new("rust".into())
        .generate_from_idl(source, Path::new("color_point.idl"), HashMap::new())
        .expect("the documented example passes shared source validation");
    let output = files
        .iter()
        .map(|file| file.content())
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!("documented_color_point", output);
}
