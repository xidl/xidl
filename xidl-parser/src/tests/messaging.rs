use crate::{hir, parser::parser_text, versioning::VersioningRecord};

const SOURCE: &str =
    include_str!("../../../bdd/features/data/photon_messaging/photon_messaging.idl");

fn lower(source: &str) -> hir::Specification {
    hir::Specification::from_typed_ast_with_properties_and_path(
        parser_text(source).expect("messaging syntax"),
        [("expand_interface".into(), false.into())].into(),
        "photon_messaging.idl",
    )
    .expect("messaging HIR")
}

#[test]
fn five_version_business_map_preserves_direct_declaration_boundaries() {
    let specification = lower(SOURCE);
    let record = VersioningRecord::analyze(&specification).unwrap().unwrap();
    assert_eq!(record.timeline().iter().count(), 5);
    let map =
        crate::versioning::map::from_spec(&specification, &record, "photon_messaging.idl").unwrap();
    assert_eq!(map.format, 2);
    assert_eq!(map.steps.len(), 4);
    let replacement = &map.steps[0].operations["Photon::Messages.legacy_send"];
    assert_ne!(
        replacement.from.as_ref().unwrap()["declaration_id"],
        replacement.to.as_ref().unwrap()["declaration_id"]
    );
    assert!(
        !map.steps[1].types.contains_key("Photon::Participant"),
        "map contains direct facts only"
    );
    let current = map.steps[2].operations["Photon::Messages.edit"]
        .to
        .as_ref()
        .unwrap();
    assert_eq!(
        current["parameters"]["header:Idempotency-Key"]["wire_name"],
        "Idempotency-Key"
    );
    assert!(
        !map.steps[0].types["Photon::Message"]
            .members
            .contains_key("internal_trace")
    );
}
