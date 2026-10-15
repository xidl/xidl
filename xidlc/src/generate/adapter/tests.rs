//! Per-rule-row unit tests for the adapter derivation.
//!
//! The depot fixture exercises every rules-table row of #324 through the
//! real pipeline: parse, analyze, intervals, merged projection, REST HIR,
//! surfaces, derive. Assertions pin classification, ordering and
//! reachability; rendering stays a later stage's business.

use super::*;

use std::collections::HashMap;

use xidl_parser::parser;
use xidl_parser::versioning::{VersionId, VersioningRecord};

/// A three-version depot whose single step v1→v2 exercises every movement
/// the rules table distinguishes, and whose second step v2→v3 stays small.
const DEPOT: &str = r#"
#pragma xidlc package adapter_depot

@version
enum ApiVersions {
    @rename("2025-01") v1,
    @rename("2025-06") v2,
    @rename("2026-01") v3,
};

/** Referenced by shared operations in every version. */
struct Sku {
    string value;
};

/** Referenced by no operation: the reachability filter must drop it. */
struct AuditTrail {
    string id;
};

/** Raised by the shared read since v1. */
@http(404)
exception ParcelNotFound {
    string sku;
};

/** Raised by the shared read since v2: the error downcast hook's subject. */
@http(422)
@added(ApiVersions::v2)
exception CarrierRejected {
    string carrier;
};

/** Outcome vocabulary with one removal and one addition. */
enum Status {
    processing,
    @removed(ApiVersions::v2) lost,
    @added(ApiVersions::v2) delayed,
};

/** Every struct-level movement of the rules table in one parcel. */
struct Parcel {
    /** Unchanged since v1: a mechanical copy in both directions. */
    Sku sku;
    /** Required until v2: the downcast hook must supply it again. */
    @removed(ApiVersions::v2) Sku return_label;
    /** Optional until v2: omitted on the way down without a decision. */
    @removed(ApiVersions::v2) @optional string color;
    /** Retired wire name of the tracking code. */
    @removed(ApiVersions::v2) @rename("code") string code_v1;
    /** Replacement wire name since v2: pairs with the retired code. */
    @added(ApiVersions::v2) @rename("code") string code_v2;
    /** Required since v2: the upcast hook must supply it. */
    @added(ApiVersions::v2) string carrier;
    /** Optional since v2: stays absent on the way up, dropped on the way down. */
    @added(ApiVersions::v2) @optional string note;
    /** Optional since v3: only the second step carries it. */
    @added(ApiVersions::v3) @optional string inspect_note;
};

interface Depot {
    /** Shared read: pairs across every step and filters by status. */
    @get(path = "/parcels/{sku}")
    Parcel get(@path string sku, @query @optional Status status)
        raises(ParcelNotFound, CarrierRejected);

    /** Shared write: the request body carries every Parcel movement. */
    @post(path = "/parcels")
    Status ship(@body Parcel parcel, @added(ApiVersions::v2) @query string lane);
};
"#;

/// A contract whose Head retired an operation: phase 1 must refuse it.
const RETIRED_OPERATION: &str = r#"
#pragma xidlc package adapter_retired_op

@version
enum ApiVersions {
    @rename("1.0") v1,
    @rename("2.0") v2,
};

interface Pets {
    @get(path = "/pets")
    string list();

    @removed(ApiVersions::v2)
    @get(path = "/legacy")
    string legacy();
};
"#;

/// A contract with a single version: nothing to pair, an empty plan.
const SINGLE_VERSION: &str = r#"
#pragma xidlc package adapter_single

@version
enum ApiVersions {
    @rename("1.0") v1,
};

struct Thing {
    string id;
};

interface Depot {
    @get(path = "/things/{id}")
    Thing get(@path string id);
};
"#;

/// Runs the full pipeline on `source`, returning the per-version surfaces
/// next to the derived plan so surface-level facts stay assertable.
fn pipeline(source: &str) -> (BTreeMap<String, VersionSurface>, IdlcResult<AdapterPlan>) {
    let typed = parser::parser_text(source).expect("fixture must parse");
    let spec = hir::Specification::from_typed_ast_with_properties(
        typed,
        HashMap::from([("hir_kind".to_string(), "http".into())]),
    );
    let record = VersioningRecord::analyze(&spec)
        .expect("versioning analysis must pass")
        .expect("fixture must declare a version enum");
    let intervals = record.member_intervals(&spec).expect("intervals");
    let mut selected: Vec<VersionId> = Vec::new();
    let mut versions: Vec<VersionDef> = Vec::new();
    for (id, def) in record.timeline().iter() {
        selected.push(id);
        versions.push(def.clone());
    }
    let merged = record.merge(&spec, &selected).expect("merged projection");
    let rest = xidl_parser::rest_hir::project(&merged).expect("rest hir");
    let surfaces = surface::collect(&merged, &rest);
    let plan = derive(&surfaces, &versions, &intervals);
    (surfaces, plan)
}

/// Derives the adapter plan from `source` through the full pipeline.
fn derive_for(source: &str) -> IdlcResult<AdapterPlan> {
    pipeline(source).1
}

fn step<'a>(plan: &'a AdapterPlan, from: &str) -> &'a StepPlan {
    plan.steps
        .iter()
        .find(|step| step.from.ident == from)
        .unwrap_or_else(|| panic!("step from {from} must exist"))
}

fn type_plan<'a>(step: &'a StepPlan, name: &str) -> &'a TypePlan {
    step.types
        .iter()
        .find(|plan| plan.name == name)
        .unwrap_or_else(|| panic!("type plan {name} must exist"))
}

fn field_names(plan: &ConversionPlan) -> Vec<(String, bool)> {
    plan.copies
        .iter()
        .map(|copy| copy.from.ident.clone())
        .map(|ident| (ident, false))
        .collect()
}

fn hook_fields(plan: &ConversionPlan) -> Vec<(String, bool)> {
    let hook = plan
        .hook
        .as_ref()
        .unwrap_or_else(|| panic!("conversion must carry a hook"));
    hook.produce
        .iter()
        .map(|field| (field.member.ident.clone(), field.ignored))
        .collect()
}

fn hook_values(plan: &ConversionPlan) -> Vec<String> {
    let hook = plan
        .hook
        .as_ref()
        .unwrap_or_else(|| panic!("conversion must carry a hook"));
    hook.consumed
        .iter()
        .map(|member| member.ident.clone())
        .collect()
}

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

#[test]
fn an_in_place_type_change_pairs_as_a_replacement() {
    let source = members_surface(
        TypeKind::Members,
        vec![member(
            "Parcel::quantity",
            "quantity",
            Some(i32_type()),
            false,
        )],
    );
    let target = members_surface(
        TypeKind::Members,
        vec![member(
            "Parcel::quantity",
            "quantity",
            Some(bool_type()),
            false,
        )],
    );
    let intervals = vec![interval_of("Parcel::quantity", 0, 2)];
    let upcast = convert_direction(Direction::Upcast, &source, &target, &intervals, 0, 1)
        .expect("replacement upcast");
    assert_eq!(
        hook_fields(&upcast),
        vec![
            ("quantity".to_string(), false),
            ("quantity".to_string(), true)
        ]
    );
    let downcast = convert_direction(Direction::Downcast, &target, &source, &intervals, 1, 0)
        .expect("replacement downcast");
    assert_eq!(
        hook_fields(&downcast),
        vec![("quantity".to_string(), false)]
    );
    assert!(downcast.copies.is_empty());
}

fn members_surface(kind: TypeKind, members: Vec<MemberSurface>) -> TypeSurface {
    TypeSurface { kind, members }
}

fn member(
    decl_path: &str,
    wire_name: &str,
    ty: Option<hir::TypeSpec>,
    optional: bool,
) -> MemberSurface {
    let ident = decl_path
        .rsplit("::")
        .next()
        .unwrap_or(decl_path)
        .to_string();
    MemberSurface {
        decl_path: decl_path.to_string(),
        ident,
        wire_name: wire_name.to_string(),
        ty,
        optional,
        location: None,
    }
}

fn i32_type() -> hir::TypeSpec {
    hir::TypeSpec::IntegerType(hir::IntegerType::I32)
}

fn bool_type() -> hir::TypeSpec {
    hir::TypeSpec::Boolean
}

fn interval_of(path: &str, from: usize, until: usize) -> MemberInterval {
    MemberInterval {
        path: path.to_string(),
        kind: xidl_parser::versioning::IntervalKind::Member,
        from,
        until,
    }
}
