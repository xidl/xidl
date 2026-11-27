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

// 中文注释验证多字节 UTF-8 偏移映射
interface Clock {
    @client-stream
    @oauth2(scopes = ["read"])
    timestamp::Instant now(in timestamp::Instant since);
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
        app.source().contains("@client-stream"),
        "original hyphenated annotation must be preserved: {}",
        app.source()
    );
    assert!(
        app.source().contains(r#"@oauth2(scopes = ["read"])"#),
        "original bracketed annotation arguments must be preserved: {}",
        app.source()
    );
    assert!(
        app.source()
            .contains("timestamp::v1::Instant now(in timestamp::v1::Instant since);"),
        "all version-bound references must be rewritten in place: {}",
        app.source()
    );
    assert_eq!(app.refs().len(), 2);
    assert!(app.refs().iter().all(|r| r.version() == Some("v1")));
    assert!(
        app.refs()
            .iter()
            .all(|r| r.qualified() == "timestamp::v1::Instant")
    );
}
