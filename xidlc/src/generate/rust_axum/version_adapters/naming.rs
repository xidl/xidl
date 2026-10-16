//! Name and type-expression assembly for the rust version-adapter module.

use std::collections::BTreeMap;

use convert_case::{Case, Casing};

use crate::error::{IdlcError, IdlcResult};
use crate::generate::rust::util::{rust_ident, rust_type};
use xidl_parser::hir;

/// Function name of a type conversion; upcasting reads the older version.
pub(crate) fn conversion_fn(upcast: bool, name: &str, from_ident: &str) -> String {
    format!(
        "{}_{}_{}_{}",
        name.to_case(Case::Snake),
        if upcast { "upcast" } else { "downcast" },
        if upcast { "from" } else { "to" },
        from_ident
    )
}

/// The loud refusal for evolving types nested inside containers.
pub(crate) fn nested_error(name: &str) -> IdlcError {
    IdlcError::fmt(format!(
        "member type {name} nests an evolving type inside a sequence, map or template; declare it as a direct member so the adapter can project it"
    ))
}

/// The error enum variant of one exception, matching the interface renderer.
pub(crate) fn error_variant(exception: &str) -> IdlcResult<String> {
    let variant = rust_ident(exception).to_case(Case::Pascal);
    if variant == "Framework" {
        return Err(IdlcError::fmt(format!(
            "exception {exception} collides with the Framework error variant; rename it"
        )));
    }
    Ok(variant)
}

/// A member's written type, wrapped when the served shape carries it optional.
pub(crate) fn optionalized_type(
    ty: &hir::TypeSpec,
    namespace: &str,
    optional: bool,
) -> IdlcResult<String> {
    let inner = versioned_type(ty, namespace)?;
    if optional {
        Ok(format!("::std::option::Option<{inner}>"))
    } else {
        Ok(inner)
    }
}

/// A member's written type in the namespace that declares it.
pub(crate) fn versioned_type(ty: &hir::TypeSpec, namespace: &str) -> IdlcResult<String> {
    match ty {
        hir::TypeSpec::ScopedName(scoped) => {
            if scoped.name.len() > 1 {
                return Err(IdlcError::fmt(
                    "types nested in modules inside a version module are not supported by version adapters yet",
                ));
            }
            let Some(last) = scoped.name.last() else {
                return Err(IdlcError::fmt("scoped type without a name"));
            };
            Ok(format!("{}::{}", namespace, rust_ident(last)))
        }
        hir::TypeSpec::SequenceType(sequence) => Ok(format!(
            "::std::vec::Vec<{}>",
            versioned_type(&sequence.ty, namespace)?
        )),
        hir::TypeSpec::MapType(map) => Ok(format!(
            "::std::collections::BTreeMap<{}, {}>",
            versioned_type(&map.key, namespace)?,
            versioned_type(&map.value, namespace)?
        )),
        other => Ok(rust_type(other)),
    }
}

/// The first evolving type nested inside containers, if any.
pub(crate) fn deep_plan_hit(ty: &hir::TypeSpec, fns: &BTreeMap<String, String>) -> Option<String> {
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
