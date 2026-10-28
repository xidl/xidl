use super::{plans, project};
use crate::hir::{ParserProperties, Specification};

#[test]
fn source_history_is_preserved_until_a_version_is_selected() {
    let source = r#"
        @version enum V { v1, v2 };
        @removed(V::v2) module policy {
            const unsigned long MASK = 0xfffffffd;
            const long WIDTH = ~MASK;
        };
        @added(V::v2) module policy {
            const unsigned long MASK = 0xfffffffc;
            const long WIDTH = ~MASK;
        };
    "#;
    let typed = crate::parser::parser_text(source).expect("versioned constant syntax");
    let raw = Specification::from_typed_ast_with_path(typed, "constants.idl")
        .expect("unprojected source HIR");
    let before = serde_json::to_value(&raw).unwrap();
    assert!(before.to_string().contains("UnaryExpr"));
    let record = crate::versioning::VersioningRecord::analyze(&raw)
        .expect("constant timeline")
        .expect("versioned contract");
    for (version, literal) in [("v1", "4294967293"), ("v2", "4294967292")] {
        let selected = record.resolve_version(version).expect("known API version");
        let projection = record.project(&raw, selected).expect("project raw source");
        let mut projection = projection;
        super::super::IntegerDeclarations::normalize(&mut projection, &Default::default())
            .expect("selected constants");
        assert!(plans(&projection)["WIDTH"].to_string().contains(literal));
        let properties = [(
            crate::versioning::API_VERSION_PROPERTY.into(),
            version.into(),
        )]
        .into_iter()
        .collect();
        assert_eq!(
            plans(&project(source, properties))["WIDTH"],
            plans(&projection)["WIDTH"]
        );
    }
    assert_eq!(serde_json::to_value(&raw).unwrap(), before);
}

#[test]
fn expression_planning_cannot_hide_references_to_retired_constants() {
    let typed = crate::parser::parser_text(
        "@version enum V { v1, v2 };\n\
         @removed(V::v2) module retired { const long EXPIRED = 2; };\n\
         const long WIDTH = retired::EXPIRED + 1;",
    )
    .expect("constant lifecycle syntax");
    let properties: ParserProperties =
        [(crate::versioning::API_VERSION_PROPERTY.into(), "v2".into())]
            .into_iter()
            .collect();
    let error = Specification::project_typed_ast_versions(typed, properties, "constants.idl")
        .expect_err("retired dependencies are checked before expression planning")
        .to_string();
    assert!(error.contains("EXPIRED"), "{error}");
}
