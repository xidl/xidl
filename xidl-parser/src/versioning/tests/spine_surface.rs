use super::{TIMELINE, spine_of};
use crate::versioning::{CellKind, Evidence};

#[test]
fn digests_are_stable_and_surface_sensitive() {
    let text = format!(
        "{TIMELINE}
struct Pet {{
    long id;
}};
"
    );
    let first = spine_of(&text).expect("versioned").surfaces;
    let second = spine_of(&text).expect("versioned").surfaces;
    assert_eq!(first, second);

    let changed = spine_of(&format!(
        "{TIMELINE}
struct Pet {{
    long id;

    @added(ApiVersions::v2025_06)
    long extra;
}};
"
    ))
    .expect("versioned")
    .surfaces;
    assert_eq!(changed[0].digest, first[0].digest, "v1 surface is intact");
    assert_ne!(changed[1].digest, first[1].digest, "v2 surface moved");
}

#[test]
fn unversioned_contracts_have_no_spine() {
    let spine = spine_of("struct Pet { long id; };");
    assert!(spine.is_none());
}

#[test]
fn single_version_timelines_have_no_correspondence() {
    let spine = spine_of(
        "
@version
enum ApiVersions {
    @rename(\"2025-01\") v1,
};

struct Item {
    string sku;
};
",
    )
    .expect("versioned");
    assert_eq!(spine.timeline.len(), 1);
    assert_eq!(spine.surfaces.len(), 1);
    assert!(
        spine.surfaces[0].digest.starts_with("md5:"),
        "digests carry their scheme: {}",
        spine.surfaces[0].digest
    );
    assert!(spine.correspondence.is_empty());
}

#[test]
fn digests_canonicalize_route_spellings_and_sense_methods() {
    let text = |binding: &str| {
        format!(
            "{TIMELINE}
interface Store {{
    {binding}
    Item get(long id);
}};

struct Item {{
    string sku;
}};
"
        )
    };
    let named = spine_of(&text("@get(path = \"/things/{id}\")")).expect("versioned");
    let positional = spine_of(&text("@get(\"/things/{id}\")")).expect("versioned");
    let slashed = spine_of(&text("@get(path = \"//things///{id}\")")).expect("versioned");
    assert_eq!(
        named.surfaces[0].digest, positional.surfaces[0].digest,
        "positional and named path spellings are one binding"
    );
    assert_eq!(
        named.surfaces[0].digest, slashed.surfaces[0].digest,
        "slash noise must not read as a different route"
    );
    let posted = spine_of(&text("@post(path = \"/things/{id}\")")).expect("versioned");
    assert_ne!(
        named.surfaces[0].digest, posted.surfaces[0].digest,
        "a method change is a wire change"
    );
}

#[test]
fn skipped_members_never_reach_the_wire() {
    let visible = format!(
        "{TIMELINE}
struct Pet {{
    long id;
}};
"
    );
    let skipped = format!(
        "{TIMELINE}
struct Pet {{
    long id;

    @skip
    string secret;
}};
"
    );
    let plain = spine_of(&visible).expect("versioned");
    let with_skip = spine_of(&skipped).expect("versioned");
    assert_eq!(
        plain.surfaces, with_skip.surfaces,
        "a @skip member is invisible to the wire surface"
    );
}

#[test]
fn skipped_members_claim_no_wire_name() {
    // The skipped declaration never serializes, so its name stays free for a
    // later interval without tripping the wire-name overlap check.
    let text = format!(
        "{TIMELINE}
struct Pet {{
    @skip
    @added(ApiVersions::v2025_01)
    long note;

    @added(ApiVersions::v2025_06)
    string note;
}};
"
    );
    let spine = spine_of(&text).expect("a skipped name stays reusable");
    let resumed = spine
        .correspondence
        .iter()
        .find(|cell| cell.wire_name == "note")
        .expect("the visible declaration is the only wire fact");
    assert_eq!(resumed.kind, CellKind::Added);
    assert_eq!(resumed.evidence, Evidence::IntervalStart);
}

#[test]
fn the_spine_serializes_as_one_reviewable_artifact() {
    // Downstream tooling consumes this JSON shape; the snapshot pins the
    // artifact schema together with one instance of every cell kind.
    let text = "
@version
@version_selector(header = \"Api-Version\")
enum ApiVersions {
    @rename(\"2025-01\") v1,
    @rename(\"2025-06\") v2,
    @rename(\"2026-01\") v3,
};

@rename_all(\"camelCase\")
struct Pet {
    long id;

    @added(ApiVersions::v2)
    @optional string note;

    @added(ApiVersions::v1)
    @removed(ApiVersions::v2)
    long weight;

    @added(ApiVersions::v1)
    @removed(ApiVersions::v2)
    long size;

    @added(ApiVersions::v2)
    @rename(\"size\")
    long size2_;

    @added(ApiVersions::v1)
    @removed(ApiVersions::v2)
    long user_id;

    @added(ApiVersions::v2)
    long userId;

    @added(ApiVersions::v1)
    @removed(ApiVersions::v2)
    long code;

    @added(ApiVersions::v3)
    @rename(\"code\")
    long code2_;

    @added(ApiVersions::v1)
    @removed(ApiVersions::v3)
    long flag;

    @added(ApiVersions::v3)
    @rename(\"flag\")
    string flag2_;

    @added(ApiVersions::v1)
    @removed(ApiVersions::v3)
    long hint;

    @added(ApiVersions::v3)
    @optional
    @rename(\"hint\")
    long hint2_;
};

interface Store {
    @get(path = \"/pets/{id}\")
    Pet find(@path string id);
};
";
    let spine = spine_of(text).expect("versioned");
    let json = serde_json::to_string_pretty(&spine).expect("the spine serializes");
    insta::assert_snapshot!("spine_artifact", json);
}

#[test]
fn digests_are_sensitive_to_http_bindings() {
    let text = |path: &str| {
        format!(
            "{TIMELINE}
interface Store {{
    @get(path = \"{path}\")
    Item get(long id);
}};

struct Item {{
    string sku;
}};
"
        )
    };
    let original = spine_of(&text("/things/{id}")).expect("versioned");
    let moved = spine_of(&text("/items/{id}")).expect("versioned");
    assert_ne!(
        original.surfaces[0].digest, moved.surfaces[0].digest,
        "a route rename must drift the frozen surface"
    );
    let streamed = spine_of(&format!(
        "{TIMELINE}
interface Store {{
    @get(path = \"/things/{{id}}\")
    @server_stream
    Item get(long id);
}};

struct Item {{
    string sku;
}};
"
    ))
    .expect("versioned");
    assert_ne!(
        original.surfaces[0].digest, streamed.surfaces[0].digest,
        "a protocol change must drift the frozen surface"
    );
}
