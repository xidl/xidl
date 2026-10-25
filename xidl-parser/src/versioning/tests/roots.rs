//! Root references retain declaration identity when projected into version modules.

use super::*;
use crate::error::ParseError;
use crate::hir::printer::IdlPrinter;
use crate::versioning::VersioningError;

#[test]
fn root_types_and_constant_expressions_stay_bound_across_merged_versions() {
    let (spec, record) = analyze(
        r#"
@version enum Versions { v1, v2 };
typedef string Identifier;
const long WIDTH = 8;
enum Kind { ready };
bitset Flags { bitfield<::WIDTH, unsigned short> value; };
module v1 { typedef long Identifier; };
module shipping {
    typedef long Identifier;
    const long WIDTH = 2;
    struct Address {
        ::Identifier id;
        sequence<::Identifier, ::WIDTH> labels;
        map<string, ::Identifier, ::WIDTH> by_tag;
        string<::WIDTH> label;
        wstring<::WIDTH> wide;
        fixed<::WIDTH, 0> amount;
        long parts[::WIDTH];
        long priority default +::WIDTH - 1;
        ::v1::Identifier legacy_code;
    };
    const long LIMIT = ::WIDTH;
    union State switch (::Kind) { case ::Kind::ready: ::Identifier id; };
    interface Deliveries {
        @post(path="/delivery") ::Identifier retain(@body ::Identifier value);
    };
};
"#,
    );
    let versions = project_ids(&record);
    for version in &versions {
        let projected = record.project(&spec, *version).expect("selected version");
        let printed = IdlPrinter::render(&projected);
        let reparsed = parse(&printed);
        assert_eq!(
            serde_json::to_value(&projected).expect("projected HIR"),
            serde_json::to_value(reparsed).expect("reparsed HIR"),
            "a published projection remains a valid ordinary IDL"
        );
    }
    let merged = record.merge(&spec, &versions).expect("merged versions");
    insta::assert_snapshot!("root_qualified_versions", IdlPrinter::render(&merged));
    let map = crate::versioning::map::from_spec(&spec, &record, "roots")
        .expect("language-neutral version facts");
    assert!(map.steps[0].types.is_empty(), "{map:#?}");
    assert!(map.steps[0].operations.is_empty(), "{map:#?}");
}

#[test]
fn removed_root_types_cannot_fall_back_to_a_surviving_inner_alias() {
    let (spec, record) = analyze(
        r#"
@version enum Versions { v1, v2 };
@removed(Versions::v2) typedef string Identifier;
module shipping {
    typedef long Identifier;
    struct Address { ::Identifier id; };
};
"#,
    );
    let versions = project_ids(&record);
    record
        .project(&spec, versions[0])
        .expect("root alias exists");
    match record.project(&spec, versions[1]) {
        Err(ParseError::Versioning(VersioningError::DanglingReference {
            decl,
            reference,
            version,
        })) => {
            assert_eq!(decl, "shipping::Address::id");
            assert_eq!(reference, "::Identifier");
            assert_eq!(version, "v2");
        }
        other => panic!("expected missing source declaration, got {other:?}"),
    }
}

#[test]
fn removed_root_exceptions_are_pruned_before_qualification_despite_inner_shadows() {
    let (spec, record) = analyze(
        r#"
@version enum Versions { v1, v2 };
@removed(Versions::v2) exception Missing { string reason; };
module shipping {
    exception Missing { long code; };
    interface Addresses {
        void read() raises(::Missing);
        readonly attribute string label raises(::Missing);
        attribute string value getraises(::Missing) setraises(Missing);
    };
};
"#,
    );
    let versions = project_ids(&record);
    let old = IdlPrinter::render(&record.project(&spec, versions[0]).expect("old exceptions"));
    assert!(old.contains("raises(::v1::Missing)"), "{old}");
    let head = IdlPrinter::render(&record.project(&spec, versions[1]).expect("pruned root"));
    assert!(!head.contains("::v2::Missing"), "{head}");
    assert!(!head.contains("getraises"), "{head}");
    assert!(head.contains("setraises(Missing)"), "{head}");
    parse(&head);
}

#[test]
fn external_names_are_not_rebased_into_a_local_version_module() {
    let (spec, record) = analyze(
        r#"
@version enum Versions { v1, v2 };
struct Address {
    external::v1::Identifier imported;
    ::external::Identifier unresolved_root;
};
"#,
    );
    let projected = record
        .project(&spec, project_ids(&record)[0])
        .expect("projection");
    let printed = IdlPrinter::render(&projected);
    assert!(
        printed.contains("external::v1::Identifier imported;"),
        "{printed}"
    );
    assert!(
        printed.contains("::external::Identifier unresolved_root;"),
        "{printed}"
    );
}

#[test]
fn constructed_typedef_tags_share_the_alias_scope_and_retirement() {
    let (spec, record) = analyze(
        r#"
@version enum Versions { v1, v2 };
@removed(Versions::v2) typedef struct Base { string id; } Alias;
struct Child : ::Base { string label; };
"#,
    );
    let versions = project_ids(&record);
    let old = record
        .project(&spec, versions[0])
        .expect("constructed root base");
    assert!(IdlPrinter::render(&old).contains("Child : ::v1::Base"));
    let merged = record
        .merge(&spec, &[versions[0]])
        .expect("merged root base");
    assert_eq!(
        serde_json::to_value(projected_module(&old)).expect("old module"),
        serde_json::to_value(projected_module(&merged)).expect("merged module")
    );
    match record.project(&spec, versions[1]) {
        Err(ParseError::Versioning(VersioningError::DanglingReference { reference, .. })) => {
            assert_eq!(reference, "::Base");
        }
        other => panic!("expected retired constructed base, got {other:?}"),
    }
}
