use super::*;
use crate::versioning::{VERSION_LAYOUT_PROPERTY, VersionLayout};

/// Fixture that exercises every supported versioning carrier.
const KITCHEN_SINK: &str = r#"
#pragma xidlc package carriers_sink
#pragma xidlc version 0.0.1

@version
enum Versions {
    v1,
    v2,
};

@added(Versions::v2)
module nested {
    @added(Versions::v2)
    struct Added {
        string id;
    };
};

@removed(Versions::v2)
module retired {
    struct Legacy {
        string id;
    };
};

struct Base {
    string id;
};

struct Derived : Base {
    @added(Versions::v2)
    string extra;

    @removed(Versions::v2)
    string legacy;
};

typedef sequence<Derived> DerivedSeq;

@added(Versions::v2)
typedef string Label;

const long LIMIT = 10;

@http(404)
exception Failure {
    string message;

    @added(Versions::v2)
    @optional string detail;
};

@added(Versions::v2)
@http(409)
exception Conflict {
    string reason;
};

union Choice switch (long) {
    case 0: @added(Versions::v2) Derived value;
    case 1: Base fallback;
};

enum Kind {
    @added(Versions::v2) Extra,
    Basic,
};

bitmask Flags {
    @added(Versions::v2) Extra,
};

bitset Mask: Base {
    bitfield<1> bit;
};

struct Forward;

interface Api {
    attribute Base current;

    @added(Versions::v2)
    readonly attribute uint16 port;

    const long MAX = 1;

    @added(Versions::v2)
    exception Inner {
        string why;
    };

    @added(Versions::v2)
    Derived fetch(@added(Versions::v2) @optional string filter) raises(Failure);

    Derived deliver(@added(Versions::v2) string note) raises(Failure, Conflict);
};
"#;

#[test]
fn projection_handles_every_supported_carrier() {
    let (spec, record) = analyze(KITCHEN_SINK);
    let versions = record
        .timeline()
        .iter()
        .map(|(id, _)| id)
        .collect::<Vec<_>>();
    let mut snapshots = Vec::new();
    for version in versions {
        let projected = record.project(&spec, version).expect("projection");
        snapshots.push(serde_json::to_string_pretty(&projected).expect("serialize"));
    }

    insta::assert_snapshot!("carriers_v1", &snapshots[0]);
    insta::assert_snapshot!("carriers_v2", &snapshots[1]);
}

#[test]
fn merged_projection_handles_every_supported_carrier() {
    let (spec, record) = analyze(KITCHEN_SINK);
    let versions = record
        .timeline()
        .iter()
        .map(|(id, _)| id)
        .collect::<Vec<_>>();
    let merged = record.merge(&spec, &versions).expect("merge");
    let json = serde_json::to_string_pretty(&merged).expect("serialize");

    insta::assert_snapshot!("carriers_merged", json);
}

#[test]
fn version_layout_reads_generation_properties() {
    assert_eq!(
        VersionLayout::from_properties(&properties(&[])).expect("layout"),
        VersionLayout::Merged
    );
    assert_eq!(
        VersionLayout::from_properties(&properties(&[(VERSION_LAYOUT_PROPERTY, "per_version")]))
            .expect("layout"),
        VersionLayout::PerVersion
    );
    assert!(matches!(
        VersionLayout::from_properties(&properties(&[(VERSION_LAYOUT_PROPERTY, "sideways")])),
        Err(crate::error::ParseError::Versioning(
            crate::versioning::VersioningError::UnknownVersionLayout { .. }
        ))
    ));
}
