use crate::error::{IdlcError, IdlcResult};
use std::collections::HashSet;
use xidl_parser::hir;

use super::attr::project_attribute;
use super::project_params::project_params;
use super::route::{
    auto_default_method_path, operation_id, parse_route_template, route_from_annotations,
};
use super::validate::{
    effective_basic_auth_realm, effective_deprecated, validate_head_constraints,
    validate_request_shape, validate_route_bindings, validate_stream_method, validate_stream_shape,
};
use super::semantics::{
    HttpStreamKind, effective_media_type, effective_security_with_origin, has_annotation,
    http_stream_config, validate_http_annotations,
};
use super::{
    HttpDocumentMetadata, HttpDocumentServer, HttpHirDocument, HttpInterface, HttpOperation,
    HttpOperationSource,
};

pub fn project(spec: &hir::Specification) -> IdlcResult<HttpHirDocument> {
    let mut ctx = ProjectionContext::default();
    ctx.collect_spec(spec, &[])?;
    Ok(HttpHirDocument {
        document: ctx.document,
        interfaces: ctx.interfaces,
    })
}

#[derive(Default)]
struct ProjectionContext {
    document: HttpDocumentMetadata,
    interfaces: Vec<HttpInterface>,
}

impl ProjectionContext {
    fn collect_spec(
        &mut self,
        spec: &hir::Specification,
        module_path: &[String],
    ) -> IdlcResult<()> {
        for def in &spec.0 {
            self.collect_def(def, module_path)?;
        }
        Ok(())
    }

    fn collect_def(&mut self, def: &hir::Definition, module_path: &[String]) -> IdlcResult<()> {
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
                if let Some(interface) = project_interface(interface, module_path)? {
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
) -> IdlcResult<Option<HttpInterface>> {
    let hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
        return Ok(None);
    };
    validate_http_annotations(
        &format!("interface '{}'", def.header.ident),
        &interface.annotations,
    )
    .map_err(IdlcError::rpc)?;

    let mut operations = Vec::new();
    if let Some(body) = &def.interface_body {
        for export in &body.0 {
            match export {
                hir::Export::OpDcl(op) => operations.push(project_operation(
                    &def.header.ident,
                    module_path,
                    &interface.annotations,
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

    Ok(Some(HttpInterface {
        name: def.header.ident.clone(),
        module_path: module_path.to_vec(),
        operations,
    }))
}

fn project_attribute_group(
    interface_name: &str,
    module_path: &[String],
    interface_annotations: &[hir::Annotation],
    attr: &hir::AttrDcl,
) -> IdlcResult<Vec<HttpOperation>> {
    validate_http_annotations(
        &format!("attribute in interface '{interface_name}'"),
        &attr.annotations,
    )
    .map_err(IdlcError::rpc)?;
    let deprecated =
        effective_deprecated(interface_annotations, &attr.annotations).map_err(IdlcError::rpc)?;
    let security = effective_security_with_origin(interface_annotations, &attr.annotations)
        .map_err(IdlcError::rpc)?;
    Ok(project_attribute(
        interface_name,
        module_path,
        attr,
        security,
        deprecated,
        has_annotation(&attr.annotations, "server_stream"),
    ))
}

fn project_operation(
    interface_name: &str,
    module_path: &[String],
    interface_annotations: &[hir::Annotation],
    op: &hir::OpDcl,
) -> IdlcResult<HttpOperation> {
    validate_http_annotations(&format!("operation '{}'", op.ident), &op.annotations)
        .map_err(IdlcError::rpc)?;
    let stream = http_stream_config(&op.annotations).map_err(IdlcError::rpc)?;
    validate_stream_shape(&op.ident, stream)?;
    let default_method = if matches!(
        stream.kind,
        Some(HttpStreamKind::Server) | Some(HttpStreamKind::Bidi)
    ) {
        super::HttpMethod::Get
    } else {
        super::HttpMethod::Post
    };
    let (method, mut route_paths) = route_from_annotations(&op.annotations, default_method)?;
    validate_stream_method(&op.ident, stream.kind, method)?;
    if route_paths.is_empty() {
        route_paths.push(auto_default_method_path(op, method)?);
    }
    let routes = route_paths
        .iter()
        .map(|path| parse_route_template(path))
        .collect::<IdlcResult<Vec<_>>>()?;
    let route_path_names = routes
        .iter()
        .map(|route| route.path_params.iter().cloned().collect::<HashSet<_>>())
        .collect::<Vec<_>>();
    let route_query_names = routes
        .iter()
        .map(|route| route.query_params.iter().cloned().collect::<HashSet<_>>())
        .collect::<Vec<_>>();

    let (
        request_params,
        request_path_params,
        request_query_params,
        request_header_params,
        request_cookie_params,
        request_body_params,
        response_params,
        response_header_params,
        response_cookie_params,
        response_body_params,
        path_binding_count,
        query_binding_count,
    ) = project_params(
        op,
        method,
        stream.kind,
        &route_path_names,
        &route_query_names,
    )?;

    validate_route_bindings(
        &op.ident,
        &routes,
        &path_binding_count,
        &query_binding_count,
    )?;
    validate_request_shape(
        &op.ident,
        stream.kind,
        &request_path_params,
        &request_query_params,
        &request_header_params,
        &request_cookie_params,
        &request_body_params,
    )?;
    validate_head_constraints(
        &op.ident,
        method,
        &response_params,
        match &op.ty {
            hir::OpTypeSpec::Void => None,
            hir::OpTypeSpec::TypeSpec(ty) => Some(ty),
        },
    )?;

    Ok(HttpOperation {
        name: op.ident.clone(),
        operation_id: operation_id(module_path, interface_name, &op.ident),
        source: HttpOperationSource::Method,
        method,
        routes,
        stream,
        request_content_type: effective_media_type(
            interface_annotations,
            &op.annotations,
            "Consumes",
        ),
        response_content_type: effective_media_type(
            interface_annotations,
            &op.annotations,
            "Produces",
        ),
        security: effective_security_with_origin(interface_annotations, &op.annotations)
            .map_err(IdlcError::rpc)?,
        basic_auth_realm: effective_basic_auth_realm(interface_annotations, &op.annotations),
        deprecated: effective_deprecated(interface_annotations, &op.annotations)
            .map_err(IdlcError::rpc)?,
        request_params,
        request_path_params,
        request_query_params,
        request_header_params,
        request_cookie_params,
        request_body_params,
        response_params,
        response_header_params,
        response_cookie_params,
        response_body_params,
        return_type: match &op.ty {
            hir::OpTypeSpec::Void => None,
            hir::OpTypeSpec::TypeSpec(ty) => Some(ty.clone()),
        },
    })
}
