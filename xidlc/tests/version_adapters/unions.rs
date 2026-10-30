use super::render;

#[test]
fn union_adapters_preserve_all_tags_and_require_explicit_retagging() {
    let files = render(
        include_str!("../../../bdd/features/data/versioned_union_tags.idl"),
        "labels.idl",
    );
    assert!(files[0].1.contains("noticeUpcastFromV1"));
    assert!(files[0].1.contains("noticeDowncastToV1"));
    insta::assert_snapshot!("union_labels_typescript_adapters", &files[0].1);
}
