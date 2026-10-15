use super::super::RestHirResult;
use super::super::model::{HttpInterface, HttpVersionSelector};
use crate::hir::VersionSelectorLocation;

/// Fails when an operation binds a request parameter under the selector's name.
///
/// The version selector owns its header or query name on every operation: a
/// parameter bound under the same name would make the generated client send
/// one value where the server expects two, and the OpenAPI document would
/// carry two same-named required parameters.
pub(crate) fn validate_selector_param_collisions(
    selector: &HttpVersionSelector,
    interfaces: &[HttpInterface],
) -> RestHirResult<()> {
    for interface in interfaces {
        for operation in &interface.operations {
            let bindings = match selector.location {
                VersionSelectorLocation::Header => &operation.http.request.header,
                VersionSelectorLocation::Query => &operation.http.request.query,
            };
            for binding in bindings {
                if binding.wire_name == selector.name {
                    return Err(format!(
                        "operation '{}' in interface '{}' binds {} parameter '{}' under the name of the @version_selector; rename the parameter or the selector",
                        operation.meta.name,
                        interface.name,
                        match selector.location {
                            VersionSelectorLocation::Header => "header",
                            VersionSelectorLocation::Query => "query",
                        },
                        binding.wire_name,
                    ));
                }
            }
        }
    }
    Ok(())
}
