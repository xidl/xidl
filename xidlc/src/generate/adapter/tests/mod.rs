//! Shared fixtures and pipeline helpers for the adapter derivation tests.

use std::collections::HashMap;

use super::*;
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

mod classify;
mod rules;
