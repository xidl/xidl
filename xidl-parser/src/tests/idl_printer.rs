use crate::{hir, parser::parser_text};

fn lower(source: &str) -> hir::Specification {
    hir::Specification::from_typed_ast_with_properties_and_path(
        parser_text(source).expect("parse source"),
        [("expand_interface".into(), false.into())].into(),
        "printer.idl",
    )
    .expect("lower source")
}

#[test]
fn versioned_includes_keep_the_callers_resolver_during_lowering() {
    struct Dependency;
    impl crate::parser::IncludeResolver for Dependency {
        fn resolve(
            &mut self,
            _parent: Option<&str>,
            path: &str,
        ) -> crate::error::ParserResult<(String, String)> {
            assert_eq!(path, "dependency/types.idl");
            Ok((
                "dependency/types.idl".into(),
                "@version enum LibraryVersions { old, current }; struct Stamp { long seconds; };"
                    .into(),
            ))
        }
    }
    let source = "#include \"dependency/types.idl@old\"\n\
                  @version enum V { v1, v2 }; struct Item { Stamp stamp; };";
    let typed =
        crate::parser::parser_text_with_resolver(source, Some("api.idl"), &mut Dependency).unwrap();
    let spec = hir::Specification::from_typed_ast_with_resolver(
        typed,
        [("expand_interface".into(), false.into())].into(),
        "api.idl",
        &mut Dependency,
    )
    .unwrap();
    let record = crate::versioning::VersioningRecord::analyze(&spec)
        .unwrap()
        .unwrap();
    for (id, _) in record.timeline().iter() {
        let projected = record.project(&spec, id).unwrap();
        let printed = hir::printer::IdlPrinter::render(&projected);
        assert!(printed.contains("struct Stamp"), "{printed}");
        assert!(!printed.contains("LibraryVersions"), "{printed}");
        assert_eq!(
            serde_json::to_value(lower(&printed)).unwrap(),
            serde_json::to_value(&projected).unwrap()
        );
    }
}

#[test]
fn versioned_rest_projection_prints_and_reparses_to_the_same_hir() {
    let source = include_str!("../../../bdd/features/data/versioned_rest.idl");
    let source = lower(source);
    let record = crate::versioning::VersioningRecord::analyze(&source)
        .expect("analyze")
        .expect("versioned");
    for (version, definition) in record.timeline().iter() {
        let projected = record.project(&source, version).expect("project");
        let printed = hir::printer::IdlPrinter::render(&projected);
        if definition.ident == "v1" {
            insta::assert_snapshot!("version_diff_projection_v1", &printed);
        }
        let reparsed = lower(&printed);
        assert_eq!(
            serde_json::to_value(&reparsed).expect("serialize reparsed"),
            serde_json::to_value(&projected).expect("serialize projected"),
            "{}: {printed}",
            definition.ident
        );
    }
}

#[test]
fn printer_ignores_documentation_annotations() {
    let source = lower("/// first\nstruct Item { /// member\n string name; };");
    let printed = hir::printer::IdlPrinter::render(&source);
    assert!(!printed.contains("first"));
    assert!(!printed.contains("member"));
    assert!(printed.contains("struct Item"));
}

#[test]
fn inline_constructs_round_trip_without_emitting_duplicate_declarations() {
    let source = lower(
        "typedef struct Inline { string value; } InlineAlias; \
         union Choice switch (long) { case 1: struct Item { long value; } item; };",
    );
    let printed = hir::printer::IdlPrinter::render(&source);
    let reparsed = lower(&printed);
    assert_eq!(
        serde_json::to_value(&reparsed).expect("serialize reparsed"),
        serde_json::to_value(&source).expect("serialize source")
    );
    assert_eq!(printed.matches("struct Inline").count(), 1);
    assert_eq!(printed.matches("struct Item").count(), 1);
}

#[test]
fn printer_preserves_defaults_and_octet_types() {
    let source = lower("struct Item { long count default 3; octet payload; };");
    let printed = hir::printer::IdlPrinter::render(&source);
    let reparsed = lower(&printed);
    assert_eq!(
        serde_json::to_value(&reparsed).expect("serialize reparsed"),
        serde_json::to_value(&source).expect("serialize source")
    );
    assert!(printed.contains("default 3"));
    assert!(printed.contains("octet payload"));
}
