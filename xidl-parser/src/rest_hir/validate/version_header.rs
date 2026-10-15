//! Header-name collision checks for the API version header.

use crate::rest_hir::semantics::{HttpApiKeyLocation, HttpSecurityRequirement};
use crate::rest_hir::{HttpInterface, HttpOperation};

use super::RestHirResult;

/// Rejects request bindings that collide with the version header.
///
/// HTTP header names are case-insensitive, so a request header parameter or
/// an `@api_key` header scheme sharing the version header's name would be
/// overwritten by the client's pinned version header.
pub(in crate::rest_hir) fn validate_version_header(
    version_header: Option<&str>,
    interfaces: &[HttpInterface],
) -> RestHirResult<()> {
    let Some(header) = version_header else {
        return Ok(());
    };
    for interface in interfaces {
        for operation in &interface.operations {
            validate_operation(interface.name.as_str(), operation, header)?;
        }
    }
    Ok(())
}

/// Rejects two request header parameters whose wire names differ only by case.
///
/// HTTP header names are case-insensitive: the transport would join or
/// overwrite the values instead of carrying two parameters.
pub(in crate::rest_hir) fn validate_unique_header_params(
    interface: &HttpInterface,
) -> RestHirResult<()> {
    for operation in &interface.operations {
        let headers = &operation.http.request.header;
        for (index, first) in headers.iter().enumerate() {
            for second in headers.iter().skip(index + 1) {
                if first
                    .wire_name
                    .eq_ignore_ascii_case(second.wire_name.as_str())
                {
                    return Err(format!(
                        "operation '{}' in interface '{}' binds header parameters '{}' and '{}' under the case-insensitive name '{}'; header names are case-insensitive",
                        operation.meta.name,
                        interface.name,
                        first.source_param,
                        second.source_param,
                        first.wire_name
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_operation(
    interface_name: &str,
    operation: &HttpOperation,
    header: &str,
) -> RestHirResult<()> {
    for binding in &operation.http.request.header {
        if binding.wire_name.eq_ignore_ascii_case(header) {
            return Err(format!(
                "operation '{}' in interface '{}' binds header parameter '{}' under the name of the @version header ('{header}'); rename the parameter or the version header",
                operation.meta.name, interface_name, binding.source_param
            ));
        }
    }
    if let Some(security) = &operation.meta.security {
        for requirement in &security.requirements {
            if let HttpSecurityRequirement::ApiKey {
                location: HttpApiKeyLocation::Header,
                name,
            } = requirement
            {
                if name.eq_ignore_ascii_case(header) {
                    return Err(format!(
                        "operation '{}' in interface '{}' declares an @api_key header scheme under the name of the @version header ('{header}'); the pinned version would overwrite the credential",
                        operation.meta.name, interface_name
                    ));
                }
            }
        }
    }
    Ok(())
}
