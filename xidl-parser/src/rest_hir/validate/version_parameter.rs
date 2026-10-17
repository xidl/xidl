//! Wire-name collision checks for version parameters and request headers.

use crate::hir::{HttpParameterLocation, VersionParameter};
use crate::rest_hir::semantics::HttpSecurityRequirement;
use crate::rest_hir::{HttpException, HttpInterface, HttpOperation};

use super::RestHirResult;

impl VersionParameter {
    pub(in crate::rest_hir) fn validate_interfaces(
        &self,
        interfaces: &[HttpInterface],
        exceptions: &[HttpException],
    ) -> RestHirResult<()> {
        for interface in interfaces {
            for operation in &interface.operations {
                self.validate_operation(&interface.name, operation)?;
                if self.location == HttpParameterLocation::Header {
                    for binding in &operation.http.response.header {
                        self.validate_response_header(&binding.wire_name, &operation.meta.name)?;
                    }
                    for reference in &operation.meta.raises {
                        if let Some(exception) = exceptions.iter().find(|exception| {
                            exception.ident == reference.ident
                                && exception.module_path == reference.module_path
                        }) {
                            for member in &exception.headers {
                                self.validate_response_header(&member.wire_name, &exception.ident)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_response_header(&self, name: &str, owner: &str) -> RestHirResult<()> {
        if self.location.names_equal(name, &self.name) {
            return Err(format!(
                "response header '{name}' on '{owner}' conflicts with the fixed @version header '{}'",
                self.name
            ));
        }
        Ok(())
    }

    fn conflicts_with(&self, location: HttpParameterLocation, name: &str) -> bool {
        (location == self.location && location.names_equal(name, &self.name))
            || (self.location == HttpParameterLocation::Cookie
                && location == HttpParameterLocation::Header
                && name.eq_ignore_ascii_case("Cookie"))
    }

    fn validate_operation(
        &self,
        interface_name: &str,
        operation: &HttpOperation,
    ) -> RestHirResult<()> {
        for (location, bindings) in [
            (
                HttpParameterLocation::Header,
                &operation.http.request.header,
            ),
            (HttpParameterLocation::Query, &operation.http.request.query),
            (
                HttpParameterLocation::Cookie,
                &operation.http.request.cookie,
            ),
        ] {
            for binding in bindings {
                if self.conflicts_with(location, &binding.wire_name) {
                    return Err(format!(
                        "operation '{}' in interface '{interface_name}' binds {} parameter '{}' in conflict with the @version parameter ('{}' in {})",
                        operation.meta.name,
                        location.as_str(),
                        binding.source_param,
                        self.name,
                        self.location.as_str()
                    ));
                }
            }
        }
        if let Some(security) = &operation.meta.security {
            for requirement in &security.requirements {
                let (location, name) = match requirement {
                    HttpSecurityRequirement::ApiKey { location, name } => {
                        (*location, name.as_str())
                    }
                    HttpSecurityRequirement::HttpBasic | HttpSecurityRequirement::HttpBearer => {
                        (HttpParameterLocation::Header, "Authorization")
                    }
                };
                if self.conflicts_with(location, name) {
                    return Err(format!(
                        "operation '{}' in interface '{interface_name}' uses {} authentication conflicting with the @version parameter ('{}'); the pinned version would overwrite the credential",
                        operation.meta.name,
                        location.as_str(),
                        self.name
                    ));
                }
            }
        }
        Ok(())
    }
}

impl HttpInterface {
    pub(in crate::rest_hir) fn validate_unique_header_params(&self) -> RestHirResult<()> {
        for operation in &self.operations {
            let headers = &operation.http.request.header;
            for (index, first) in headers.iter().enumerate() {
                for second in headers.iter().skip(index + 1) {
                    if HttpParameterLocation::Header
                        .names_equal(&first.wire_name, &second.wire_name)
                    {
                        return Err(format!(
                            "operation '{}' in interface '{}' binds header parameters '{}' and '{}' under the case-insensitive name '{}'; header names are case-insensitive",
                            operation.meta.name,
                            self.name,
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
}
