//! Name and type-expression assembly for the adapter renderer.

use std::collections::BTreeMap;

use convert_case::{Case, Casing};

use crate::error::{IdlcError, IdlcResult};
use crate::generate::typescript::definition::contexts::TsType;
use crate::generate::typescript::definition::method::TypeRefTarget;
use crate::generate::typescript::definition::type_expr::ts_type_for_type_spec;
use xidl_parser::hir;

use super::context::TypeRoot;

/// Function name of a type conversion; upcasting reads the older version.
pub(crate) fn conversion_fn(upcast: bool, name: &str, from_ident: &str) -> String {
    let pascal = name.to_case(Case::Pascal);
    let from_pascal = from_ident.to_case(Case::Pascal);
    if upcast {
        format!("upcast{pascal}From{from_pascal}")
    } else {
        format!("downcast{pascal}To{from_pascal}")
    }
}

/// The versioned type reference a named declaration resolves to.
pub(crate) fn versioned_scoped(root: TypeRoot, namespace: &str, name: &str) -> TsType {
    let module = match root {
        TypeRoot::Models => "types",
        TypeRoot::Iface => "ifaceTypes",
    };
    TsType::ScopedName(format!("{module}.{namespace}.{name}"))
}

/// A member's written type in the namespace that declares it.
pub(crate) fn member_type(ty: &hir::TypeSpec, namespace: &str) -> IdlcResult<TsType> {
    if let hir::TypeSpec::ScopedName(scoped) = ty {
        if scoped.name.len() > 1 {
            return Err(IdlcError::fmt(
                "types nested in modules inside a version module are not supported by version adapters yet",
            ));
        }
    }
    Ok(ts_type_for_type_spec(
        ty,
        std::slice::from_ref(&namespace.to_string()),
        TypeRefTarget::Client,
    ))
}

/// The name a copy's source type resolves to when it has its own conversion.
pub(crate) fn nested_plan(
    ty: &Option<hir::TypeSpec>,
    fns: &BTreeMap<String, String>,
) -> IdlcResult<Option<String>> {
    let Some(spec) = ty else {
        return Ok(None);
    };
    match spec {
        hir::TypeSpec::ScopedName(scoped) => {
            let Some(last) = scoped.name.last() else {
                return Ok(None);
            };
            if fns.contains_key(last) {
                return Ok(Some(fns[last].clone()));
            }
            deep_plan_hit(spec, fns)
                .map(|nested| Err(nested_error(&nested)))
                .unwrap_or(Ok(None))
        }
        spec => deep_plan_hit(spec, fns)
            .map(|nested| Err(nested_error(&nested)))
            .unwrap_or(Ok(None)),
    }
}

fn nested_error(name: &str) -> IdlcError {
    IdlcError::fmt(format!(
        "member type {name} nests an evolving type inside a sequence, map or template; declare it as a direct member so the adapter can project it"
    ))
}

/// The first evolving type nested inside containers, if any.
fn deep_plan_hit(ty: &hir::TypeSpec, fns: &BTreeMap<String, String>) -> Option<String> {
    match ty {
        hir::TypeSpec::ScopedName(scoped) => scoped
            .name
            .last()
            .filter(|name| fns.contains_key(*name))
            .cloned(),
        hir::TypeSpec::SequenceType(sequence) => deep_plan_hit(&sequence.ty, fns),
        hir::TypeSpec::MapType(map) => deep_plan_hit(&map.value, fns),
        hir::TypeSpec::TemplateType(template) => {
            template.args.iter().find_map(|arg| deep_plan_hit(arg, fns))
        }
        _ => None,
    }
}

pub(crate) fn verb_word(upcast: bool) -> &'static str {
    if upcast { "Upcast" } else { "Downcast" }
}

pub(crate) fn verb_lower(upcast: bool) -> &'static str {
    if upcast { "upcast" } else { "downcast" }
}

pub(crate) fn prep_word(upcast: bool) -> &'static str {
    if upcast { "From" } else { "To" }
}
