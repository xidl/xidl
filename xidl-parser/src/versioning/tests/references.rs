//! Reference bookkeeping across a projection: dangling references, pruned
//! exception sets and root-qualification rewriting.

use super::*;
use crate::error::ParseError;
use crate::versioning::VersioningError;

#[test]
fn projection_reports_references_into_dropped_subtrees() {
    let (spec, record) = analyze(
        r#"
@version
enum Versions { v1, v2 };

@added(Versions::v2)
module Extras {
    struct Note { long x; };
};

struct Pet {
    string id;
    Extras::Note note;
};
"#,
    );
    let versions = project_ids(&record);

    record
        .project(&spec, versions[1])
        .expect("v2 keeps the module");
    match record.project(&spec, versions[0]) {
        Err(ParseError::Versioning(VersioningError::DanglingReference {
            decl,
            reference,
            version,
        })) => {
            assert!(decl.contains("Pet"));
            assert_eq!(reference, "Extras::Note");
            assert_eq!(version, "v1");
        }
        other => panic!("expected a dangling reference error, got {other:?}"),
    }
}

#[test]
fn projection_reports_references_to_the_version_enum() {
    let (spec, record) = analyze(
        r#"
@version
enum ApiVersions { v1, v2 };

struct Uses { ApiVersions v; };
"#,
    );
    let versions = project_ids(&record);

    match record.project(&spec, versions[0]) {
        Err(ParseError::Versioning(VersioningError::DanglingReference { reference, .. })) => {
            assert_eq!(reference, "ApiVersions")
        }
        other => panic!("expected a dangling reference error, got {other:?}"),
    }
}

#[test]
fn projection_matches_qualified_references_across_modules() {
    let (spec, record) = analyze(
        r#"
@version
enum Versions { v1, v2 };

module A {
    @added(Versions::v2)
    struct Foo { long x; };
};

module B {
    struct Foo { long y; };
};

struct Pet { A::Foo foo; };
"#,
    );
    let versions = project_ids(&record);

    record.project(&spec, versions[1]).expect("v2 keeps A::Foo");
    match record.project(&spec, versions[0]) {
        Err(ParseError::Versioning(VersioningError::DanglingReference { reference, .. })) => {
            assert_eq!(reference, "A::Foo")
        }
        other => panic!("expected a dangling reference error, got {other:?}"),
    }
}

#[test]
fn projection_prunes_raises_into_dropped_subtrees() {
    let (spec, record) = analyze(
        r#"
@version
enum Versions { v1, v2 };

@removed(Versions::v2)
module Late {
    @http(409)
    exception Conflict { string reason; };
};

interface Pets {
    void adopt() raises(Late::Conflict);
};
"#,
    );
    let versions = project_ids(&record);

    let v1 = record
        .project(&spec, versions[0])
        .expect("v1 keeps the exception");
    assert_eq!(raised_exceptions(&v1), vec!["Late::Conflict"]);
    let v2 = record
        .project(&spec, versions[1])
        .expect("v2 prunes the entry");
    assert!(raised_exceptions(&v2).is_empty());
}

#[test]
fn projection_relativizes_const_values_and_union_labels() {
    let (spec, record) = analyze(
        r#"
@version
enum Versions { v1, v2 };

enum Color { RED, GREEN };

const long FAVOURITE = ::Color::RED;

union Paint switch (Color) {
    case ::Color::GREEN: long g;
};
"#,
    );
    let versions = project_ids(&record);
    let projected = record.project(&spec, versions[0]).expect("projection");
    let json = serde_json::to_string(&projected).expect("serialize");

    assert!(
        !json.contains("\"is_root\":true"),
        "root references must be relativized: {json}"
    );
}
