//! Pairs the operations both versions of a step declare.
//!
//! Requests only ever flow upward; responses flow downward through the
//! return type's own plan. Each direction's full `raises` list travels in
//! the plan so language renderers can project the per-method error enums.

use std::collections::BTreeSet;

use crate::error::IdlcResult;

use super::OperationPlan;
use super::classify::{Direction, convert_direction};
use super::surface::{MemberSurface, ParameterSurface, TypeKind, TypeSurface, VersionSurface};
use xidl_parser::versioning::MemberInterval;

/// Pairs the operations both versions declare.
pub(crate) fn pair_operations(
    from: &VersionSurface,
    to: &VersionSurface,
    intervals: &[MemberInterval],
    from_index: usize,
    to_index: usize,
) -> IdlcResult<Vec<OperationPlan>> {
    let mut plans = Vec::new();
    for (interface_name, from_interface) in &from.interfaces {
        let Some(to_interface) = to.interfaces.get(interface_name) else {
            continue;
        };
        for (name, from_operation) in &from_interface.operations {
            let Some(to_operation) = to_interface.operations.get(name) else {
                continue;
            };
            let from_raises: BTreeSet<&str> =
                from_operation.raises.iter().map(String::as_str).collect();
            let raises_added = to_operation
                .raises
                .iter()
                .filter(|exception| !from_raises.contains(exception.as_str()))
                .cloned()
                .collect();
            let from_params = synthetic_struct(&from_operation.params);
            let to_params = synthetic_struct(&to_operation.params);
            let upcast = convert_direction(
                Direction::Upcast,
                &from_params,
                &to_params,
                intervals,
                from_index,
                to_index,
            )?;
            plans.push(OperationPlan {
                interface: interface_name.clone(),
                name: name.clone(),
                upcast,
                return_type: to_operation.return_type.clone(),
                raises_from: from_operation.raises.clone(),
                raises_to: to_operation.raises.clone(),
                raises_added,
            });
        }
    }
    Ok(plans)
}

/// The rules table treats an operation's request parameters as one synthetic
/// struct, so they classify exactly like members do.
fn synthetic_struct(params: &[ParameterSurface]) -> TypeSurface {
    TypeSurface {
        kind: TypeKind::Members,
        members: params
            .iter()
            .map(|parameter| MemberSurface {
                decl_path: parameter.decl_path.clone(),
                ident: parameter.ident.clone(),
                wire_name: parameter.wire_name.clone(),
                ty: Some(parameter.ty.clone()),
                optional: parameter.optional,
                location: Some(parameter.location),
            })
            .collect(),
    }
}
