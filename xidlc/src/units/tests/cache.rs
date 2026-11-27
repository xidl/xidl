use super::support::TempDir;
use crate::units::UnitGraph;
use std::collections::BTreeMap;

#[test]
fn unit_exposes_the_typed_ast_matching_its_prepared_source() {
    let dir = TempDir::new("typed-ast-cache");
    dir.write(
        "timestamp/index.idl",
        "@version\nenum ApiVersions { v1, v2 };\nstruct Instant { string at; };\n",
    );
    let app = dir.write(
        "app.idl",
        "import \"timestamp@v1\";\nstruct Event { timestamp::Instant at; };\n",
    );

    let graph = UnitGraph::build(&[app], &BTreeMap::new()).expect("graph builds");
    let app = graph
        .units()
        .iter()
        .find(|unit| unit.prefix() == "app")
        .expect("app unit");
    let typed_ast = serde_json::to_string(app.typed_ast()).expect("serialize typed ast");

    assert_eq!(app.refs()[0].qualified(), "timestamp::v1::Instant");
    assert!(
        typed_ast.contains("timestamp::v1::Instant"),
        "typed AST must match the prepared source: {typed_ast}"
    );
}
