//! One unit test per rules-table row, through the real pipeline.

use super::*;

#[test]
fn unchanged_members_copy_mechanically_in_both_directions() {
    let plan = derive_for(DEPOT).expect("depot must derive");
    let first = step(&plan, "v1");
    let parcel = type_plan(first, "Parcel");
    let upcast = parcel.upcast.as_ref().expect("Parcel upcast");
    let downcast = parcel.downcast.as_ref().expect("Parcel downcast");
    assert_eq!(field_names(upcast), vec![("sku".to_string(), false)]);
    assert_eq!(field_names(downcast), vec![("sku".to_string(), false)]);
    let copy = &upcast.copies[0];
    assert_eq!(copy.from.decl_path, "Parcel::sku");
    assert_eq!(copy.to.decl_path, copy.from.decl_path);
    let sku = type_plan(first, "Sku");
    let sku_up = sku.upcast.as_ref().expect("Sku upcast");
    let sku_down = sku.downcast.as_ref().expect("Sku downcast");
    assert!(sku_up.hook.is_none() && sku_down.hook.is_none());
}

#[test]
fn removed_members_pair_per_optionality_and_direction() {
    let plan = derive_for(DEPOT).expect("depot must derive");
    let parcel = type_plan(step(&plan, "v1"), "Parcel");

    let upcast = parcel.upcast.as_ref().expect("Parcel upcast");
    assert_eq!(
        hook_fields(upcast),
        vec![
            ("return_label".to_string(), true),
            ("color".to_string(), true),
            ("code_v1".to_string(), true),
            ("code_v2".to_string(), false),
            ("carrier".to_string(), false),
        ]
    );

    let downcast = parcel.downcast.as_ref().expect("Parcel downcast");
    assert_eq!(
        hook_fields(downcast),
        vec![
            ("return_label".to_string(), false),
            ("code_v1".to_string(), false),
        ]
    );
}

#[test]
fn added_optional_members_stay_absent_without_decisions() {
    let plan = derive_for(DEPOT).expect("depot must derive");
    let parcel = type_plan(step(&plan, "v1"), "Parcel");
    let upcast = parcel.upcast.as_ref().expect("Parcel upcast");
    let copied: Vec<String> = upcast
        .copies
        .iter()
        .map(|copy| copy.from.ident.clone())
        .collect();
    let hooked: Vec<String> = hook_fields(upcast)
        .into_iter()
        .map(|(ident, _)| ident)
        .collect();
    assert!(!copied.contains(&"note".to_string()));
    assert!(!hooked.contains(&"note".to_string()));

    let downcast = parcel.downcast.as_ref().expect("Parcel downcast");
    let down_copied: Vec<String> = downcast
        .copies
        .iter()
        .map(|copy| copy.from.ident.clone())
        .collect();
    assert!(!down_copied.contains(&"note".to_string()));
}

#[test]
fn enum_movements_hook_the_reading_direction_only() {
    let plan = derive_for(DEPOT).expect("depot must derive");
    let status = type_plan(step(&plan, "v1"), "Status");
    assert_eq!(status.kind, TypeKind::Enum);

    let upcast = status.upcast.as_ref().expect("Status upcast");
    assert_eq!(field_names(upcast), vec![("processing".to_string(), false)]);
    assert_eq!(hook_values(upcast), vec!["lost".to_string()]);
    assert!(
        upcast
            .hook
            .as_ref()
            .is_some_and(|hook| hook.produce.is_empty()),
        "enum hooks carry no result fields"
    );

    let downcast = status.downcast.as_ref().expect("Status downcast");
    assert_eq!(
        field_names(downcast),
        vec![("processing".to_string(), false)]
    );
    assert_eq!(hook_values(downcast), vec!["delayed".to_string()]);
}

#[test]
fn second_step_carries_only_its_own_movements() {
    let plan = derive_for(DEPOT).expect("depot must derive");
    assert_eq!(plan.steps.len(), 2, "three versions make two steps");
    let second = step(&plan, "v2");
    assert_eq!(second.to.ident, "v3", "the head closes the timeline");

    let parcel = type_plan(second, "Parcel");
    let upcast = parcel.upcast.as_ref().expect("Parcel upcast");
    assert!(
        upcast.hook.is_none(),
        "an added optional member makes no decision on either side"
    );
    let downcast = parcel.downcast.as_ref().expect("Parcel downcast");
    assert!(downcast.hook.is_none());

    let status = type_plan(second, "Status");
    let upcast = status.upcast.as_ref().expect("Status upcast");
    assert_eq!(
        field_names(upcast),
        vec![
            ("processing".to_string(), false),
            ("delayed".to_string(), false)
        ]
    );
    assert!(upcast.hook.is_none());

    let carrier = type_plan(second, "CarrierRejected");
    assert!(
        carrier.upcast.is_none(),
        "raised exceptions are never upcast"
    );
    assert!(carrier.downcast.is_some());
}

#[test]
fn reachability_drops_types_no_operation_touches() {
    let plan = derive_for(DEPOT).expect("depot must derive");
    for step in &plan.steps {
        assert!(
            step.types.iter().all(|plan| plan.name != "AuditTrail"),
            "AuditTrail is referenced by no operation and must be dropped"
        );
    }
}

#[test]
fn retired_raised_exception_reaches_the_operation_plan() {
    let plan = derive_for(DEPOT).expect("depot must derive");
    let operations = &step(&plan, "v1").operations;
    let get = operations
        .iter()
        .find(|operation| operation.name == "get")
        .expect("shared operation get");
    assert_eq!(get.raises_added, vec!["CarrierRejected".to_string()]);
    let ship = operations
        .iter()
        .find(|operation| operation.name == "ship")
        .expect("shared operation ship");
    assert!(ship.raises_added.is_empty());
}

#[test]
fn added_required_parameters_hook_the_request_upcast() {
    let plan = derive_for(DEPOT).expect("depot must derive");
    let operations = &step(&plan, "v1").operations;
    let ship = operations
        .iter()
        .find(|operation| operation.name == "ship")
        .expect("shared operation ship");
    assert_eq!(ship.interface, "Depot");
    let upcast = ship.upcast.as_ref().expect("ship upcast");
    assert_eq!(hook_fields(upcast), vec![("lane".to_string(), false)]);
    let copied: Vec<String> = upcast
        .copies
        .iter()
        .map(|copy| copy.from.ident.clone())
        .collect();
    assert_eq!(copied, vec!["parcel".to_string()]);
}

#[test]
fn a_version_may_not_outlive_an_operation_the_head_removed() {
    let error = derive_for(RETIRED_OPERATION).expect_err("retired operations must fail");
    let message = error.to_string();
    assert!(message.contains("legacy"), "{message}");
    assert!(message.contains("1.0"), "{message}");
    assert!(message.contains("2.0"), "{message}");
}

#[test]
fn a_single_version_plan_has_nothing_to_pair() {
    let plan = derive_for(SINGLE_VERSION).expect("single version must derive");
    assert!(plan.steps.is_empty());
}

#[test]
fn surfaces_carry_request_locations_and_wire_names() {
    let (surfaces, _) = pipeline(DEPOT);
    let v1 = surfaces.get("v1").expect("v1 surface");
    let depot = v1.interfaces.get("Depot").expect("Depot interface");
    let get = depot.operations.get("get").expect("get operation");
    let locations: Vec<(String, String)> = get
        .params
        .iter()
        .map(|parameter| (parameter.ident.clone(), format!("{:?}", parameter.location)))
        .collect();
    assert_eq!(
        locations,
        vec![
            ("sku".to_string(), "Path".to_string()),
            ("status".to_string(), "Query".to_string()),
        ]
    );
    assert_eq!(get.params[0].wire_name, "sku");
    let ship = depot.operations.get("ship").expect("ship operation");
    let body = &ship.params[0];
    assert_eq!(
        format!("{:?}", body.location),
        "Body",
        "the un-annotated write parameter rides in the body"
    );
}

#[test]
fn deriving_without_versions_is_rejected() {
    let error = derive(&Default::default(), &[], &[]).expect_err("versions are required");
    assert!(error.to_string().contains("at least one declared version"));
}
