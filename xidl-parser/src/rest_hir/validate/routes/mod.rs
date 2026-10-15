use std::collections::HashMap;

use super::http_method_name;
use crate::rest_hir::{HttpOperation, RestHirResult};

/// Fails when one interface binds the same HTTP method and path twice.
///
/// Every HTTP target projects one interface into one route table, so a repeated
/// binding cannot be resolved without a runtime decision. Running the check
/// during projection makes it apply to every generated artifact and to each
/// projected API version, while identical routes in different interfaces stay
/// legal because each interface is mounted separately.
pub(crate) fn validate_unique_route_bindings(
    interface_name: &str,
    operations: &[HttpOperation],
) -> RestHirResult<()> {
    let mut bindings = HashMap::<String, String>::new();
    for operation in operations {
        for route in &operation.meta.routes {
            let key = format!("{} {}", http_method_name(operation.meta.method), route.path);
            if let Some(previous) = bindings.insert(key.clone(), operation.meta.name.clone()) {
                return Err(format!(
                    "duplicate HTTP route binding in interface '{interface_name}': {key} \
                     (operations: {previous}, {}); give each operation a distinct path, or use \
                     an @http union for content negotiation on one route",
                    operation.meta.name
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
