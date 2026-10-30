use xidl_parser::hir::{Specification, printer::IdlPrinter};
use xidl_parser::lint::{AnnotationDefinition, DiagnosticCode, Severity};
use xidl_parser::parser::parser_text;
use xidl_parser::rest_hir::ProjectedHir;
use xidl_parser::versioning::VERSION_LAYOUT_PROPERTY;

#[test]
fn dds_annotations_keep_source_data_without_builtin_semantics() {
    for (name, source) in [
        ("dds_service", "@dds_service interface Service {};"),
        (
            "dds_request_topic",
            "interface Service { @DDSRequestTopic(\"requests\") void ping(); };",
        ),
        (
            "dds_reply_topic",
            "interface Service { @DDSReplyTopic(\"replies\") void ping(); };",
        ),
    ] {
        assert!(AnnotationDefinition::lookup(name).is_none());
        let typed = parser_text(source).expect("concrete IDL syntax");
        let diagnostics = xidl_parser::lint::lint(&typed);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].code, DiagnosticCode::UnknownAnnotation);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        let spec = Specification::from(typed);
        assert_eq!(spec.0.len(), 1);
        let printed = IdlPrinter::render(&spec);
        let reparsed = Specification::from(parser_text(&printed).expect("source round trip"));
        assert_eq!(
            serde_json::to_value(&spec).unwrap(),
            serde_json::to_value(&reparsed).unwrap(),
            "{printed}"
        );
    }
}

#[test]
fn version_layouts_preserve_only_the_selected_interface_declarations() {
    let source = r#"
        @version enum Versions { v1, v2, };
        module accounts {
            interface Ledger {
                void ping();
                @removed(Versions::v2) long legacy();
                @added(Versions::v2) long revise(in long id, inout long revision);
                @removed(Versions::v2) readonly attribute string label;
                @added(Versions::v2) attribute string title;
            };
        };
    "#;
    for layout in ["merged", "per_version"] {
        let projections = Specification::project_typed_ast_versions(
            parser_text(source).expect("versioned source"),
            [(VERSION_LAYOUT_PROPERTY.to_string(), layout.into())].into(),
            "accounts.idl",
        )
        .expect("version projections");
        let mut versions = 0;
        for projection in projections {
            let ProjectedHir::Rpc(spec) = projection.hir else {
                panic!("source HIR expected");
            };
            for definition in spec.0 {
                if matches!(definition, xidl_parser::hir::Definition::Pragma(_)) {
                    continue;
                }
                let xidl_parser::hir::Definition::ModuleDcl(module) = definition else {
                    panic!("one source module per selected version");
                };
                let printed = IdlPrinter::render(&Specification(module.definition));
                assert_eq!(printed.matches("interface Ledger").count(), 1);
                assert!(!printed.contains("struct "), "{printed}");
                assert!(!printed.contains("union "), "{printed}");
                assert!(!printed.contains("const "), "{printed}");
                assert!(printed.contains("void ping()"), "{printed}");
                assert_eq!(printed.contains("legacy()"), module.ident == "v1");
                assert_eq!(
                    printed.contains("attribute string label"),
                    module.ident == "v1"
                );
                assert_eq!(printed.contains("long revise("), module.ident == "v2");
                assert_eq!(
                    printed.contains("attribute string title"),
                    module.ident == "v2"
                );
                versions += 1;
            }
        }
        assert_eq!(versions, 2);
    }
}
