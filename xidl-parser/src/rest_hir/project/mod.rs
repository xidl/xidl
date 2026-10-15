use crate::error::{ParseError, ParserResult};
use crate::hir;

use super::attr::project_attribute;

mod operation;
use super::model::{HttpApiVersion, HttpException, HttpUnion, HttpVersionSelector};
use super::semantics::{
    effective_cors, effective_security_with_origin, has_annotation, validate_http_annotations,
};
use super::validate::{
    effective_deprecated, validate_selector_param_collisions, validate_unique_route_bindings,
};
use super::{
    HttpDocumentMetadata, HttpDocumentServer, HttpInterface, HttpOperation, RestHirDocument,
};

pub fn project(spec: &hir::Specification) -> ParserResult<RestHirDocument> {
    let mut ctx = ProjectionContext::default();
    // Exceptions and `@http` unions first: operations resolve against them.
    ctx.collect_exceptions(&spec.0, &[])?;
    ctx.document.http_unions = HttpUnion::collect(&spec.0)?;
    ctx.collect_spec(spec, &[])?;
    ctx.document.exceptions = ctx.exceptions;
    if let Some(selector) = &ctx.document.version_selector {
        validate_selector_param_collisions(selector, &ctx.interfaces).map_err(parse_err)?;
    }
    Ok(RestHirDocument {
        spec: spec.clone(),
        document: ctx.document,
        interfaces: ctx.interfaces,
    })
}

fn parse_err(message: String) -> ParseError {
    ParseError::Message(message)
}

#[derive(Default)]
struct ProjectionContext {
    document: HttpDocumentMetadata,
    exceptions: Vec<HttpException>,
    interfaces: Vec<HttpInterface>,
}

impl ProjectionContext {
    fn collect_exceptions(
        &mut self,
        definitions: &[hir::Definition],
        module_path: &[String],
    ) -> ParserResult<()> {
        for def in definitions {
            match def {
                hir::Definition::ModuleDcl(module) => {
                    let mut next = module_path.to_vec();
                    next.push(module.ident.clone());
                    self.collect_exceptions(&module.definition, &next)?;
                }
                hir::Definition::ExceptDcl(except) => {
                    self.exceptions
                        .push(HttpException::from_hir(except, module_path)?);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn collect_spec(
        &mut self,
        spec: &hir::Specification,
        module_path: &[String],
    ) -> ParserResult<()> {
        for def in &spec.0 {
            self.collect_def(def, module_path)?;
        }
        Ok(())
    }

    fn collect_def(&mut self, def: &hir::Definition, module_path: &[String]) -> ParserResult<()> {
        match def {
            hir::Definition::ModuleDcl(module) => {
                let mut next = module_path.to_vec();
                next.push(module.ident.clone());
                for def in &module.definition {
                    self.collect_def(def, &next)?;
                }
            }
            hir::Definition::Pragma(pragma) => self.apply_pragma(pragma),
            hir::Definition::InterfaceDcl(interface) => {
                if let Some(interface) = project_interface(
                    interface,
                    module_path,
                    &self.exceptions,
                    &self.document.http_unions,
                )? {
                    self.interfaces.push(interface);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn apply_pragma(&mut self, pragma: &hir::Pragma) {
        match pragma {
            hir::Pragma::XidlcPackage(value) if !value.is_empty() => {
                self.document.package = Some(normalize_pragma_scalar(value));
            }
            hir::Pragma::XidlcOpenApiVersion(value) if !value.is_empty() => {
                self.document.version = Some(normalize_pragma_scalar(value));
            }
            hir::Pragma::XidlcOpenApiService {
                base_url,
                description,
            } if !base_url.is_empty() => {
                self.document.servers.push(HttpDocumentServer {
                    base_url: base_url.clone(),
                    description: description.clone(),
                });
            }
            hir::Pragma::XidlcApiVersions { selector, versions } => {
                self.document.api_versions = versions
                    .iter()
                    .map(|version| HttpApiVersion {
                        ident: version.ident.clone(),
                        value: version.value.clone(),
                    })
                    .collect();
                self.document.version_selector = selector.as_ref().and_then(|selector| {
                    Some(HttpVersionSelector {
                        location: match selector.location.as_str() {
                            "query" => hir::VersionSelectorLocation::Query,
                            "header" => hir::VersionSelectorLocation::Header,
                            _ => return None,
                        },
                        name: selector.name.clone(),
                    })
                });
            }
            _ => {}
        }
    }
}

fn normalize_pragma_scalar(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('=')
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn project_interface(
    interface: &hir::InterfaceDcl,
    module_path: &[String],
    exceptions: &[HttpException],
    http_unions: &[HttpUnion],
) -> ParserResult<Option<HttpInterface>> {
    let hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
        return Ok(None);
    };
    validate_http_annotations(
        &format!("interface '{}'", def.header.ident),
        &interface.annotations,
    )
    .map_err(ParseError::Message)?;

    let mut operations = Vec::new();
    if let Some(body) = &def.interface_body {
        for export in &body.0 {
            match export {
                hir::Export::OpDcl(op) => operations.push(operation::project_operation(
                    &def.header.ident,
                    module_path,
                    &interface.annotations,
                    exceptions,
                    http_unions,
                    op,
                )?),
                hir::Export::AttrDcl(attr) => operations.extend(project_attribute_group(
                    &def.header.ident,
                    module_path,
                    &interface.annotations,
                    attr,
                )?),
                _ => {}
            }
        }
    }

    validate_unique_route_bindings(&def.header.ident, &operations).map_err(parse_err)?;

    Ok(Some(HttpInterface {
        name: def.header.ident.clone(),
        module_path: module_path.to_vec(),
        operations,
    }))
}

pub(super) fn project_attribute_group(
    interface_name: &str,
    module_path: &[String],
    interface_annotations: &[hir::Annotation],
    attr: &hir::AttrDcl,
) -> ParserResult<Vec<HttpOperation>> {
    validate_http_annotations(
        &format!("attribute in interface '{interface_name}'"),
        &attr.annotations,
    )
    .map_err(ParseError::Message)?;
    if has_annotation(&attr.annotations, "cors") {
        return Err(ParseError::Message(format!(
            "attribute in interface '{interface_name}': @cors is only supported on interfaces and methods"
        )));
    }
    let cors = effective_cors(interface_annotations, &[]).map_err(ParseError::Message)?;
    let deprecated = effective_deprecated(interface_annotations, &attr.annotations)
        .map_err(ParseError::Message)?;
    let security = effective_security_with_origin(interface_annotations, &attr.annotations)
        .map_err(ParseError::Message)?;
    let mut operations = project_attribute(
        interface_name,
        module_path,
        attr,
        security,
        deprecated,
        has_annotation(&attr.annotations, "server_stream"),
    );
    for operation in &mut operations {
        operation.meta.cors = cors.clone();
    }
    Ok(operations)
}
