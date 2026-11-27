use super::support::TempDir;
use crate::units::UnitGraph;
use std::collections::BTreeMap;

const VERSIONED_TIMESTAMP: &str = "@version\nenum ApiVersions {\n    v1, v2\n};\n\n\
                                  struct Instant { string at; };\n";

#[test]
fn normalized_annotations_do_not_hide_version_bound_references() {
    let dir = TempDir::new("normalized-refs");
    dir.write("timestamp/index.idl", VERSIONED_TIMESTAMP);
    let app = dir.write(
        "app.idl",
        r#"import "timestamp@v1";

interface Clock {
    @client-stream
    @oauth2(scopes = ["read"])
    timestamp::Instant now();
};
"#,
    );

    let graph = UnitGraph::build(&[app], &BTreeMap::new()).expect("graph builds");
    let app = graph
        .units()
        .iter()
        .find(|unit| unit.prefix() == "app")
        .expect("app unit");

    assert!(
        app.source().contains("timestamp::v1::Instant"),
        "source: {}",
        app.source()
    );
    assert_eq!(app.refs().len(), 1);
    assert_eq!(app.refs()[0].version(), Some("v1"));
    assert_eq!(app.refs()[0].qualified(), "timestamp::v1::Instant");
}
