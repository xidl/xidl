use crate::error::{ParseError, ParserResult};
use crate::hir;
use std::collections::HashSet;

use super::attr::project_attribute;
use super::project_params::project_params;
use super::route::{
    auto_default_method_path, operation_id, parse_route_template, route_from_annotations,
};
use super::semantics::{
    HttpStreamKind, effective_media_type, effective_security_with_origin, has_annotation,
    http_stream_config, validate_http_annotations,
};
use super::validate::{
    effective_basic_auth_realm, effective_deprecated, validate_head_constraints,
    validate_request_shape, validate_route_bindings, validate_stream_method, validate_stream_shape,
};
use super::{
    HttpDocumentMetadata, HttpDocumentServer, HttpInterface, HttpOperation, HttpOperationSource,
    RestHirDocument,
};

pub fn project(spec: &hir::Specification) -> ParserResult<RestHirDocument> {
    let mut ctx = ProjectionContext::default();
    ctx.collect_spec(spec, &[])?;
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
    interfaces: Vec<HttpInterface>,
}

impl ProjectionContext {
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
) -> ParserResult<Vec<HttpOperation>> {
    validate_http_annotations(
        &format!("attribute in interface '{interface_name}'"),
        &attr.annotations,
    )
    .map_err(ParseError::Message)?;
    let deprecated = effective_deprecated(interface_annotations, &attr.annotations)
        .map_err(ParseError::Message)?;
    let security = effective_security_with_origin(interface_annotations, &attr.annotations)
        .map_err(ParseError::Message)?;
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
) -> ParserResult<HttpOperation> {
    validate_http_annotations(&format!("operation '{}'", op.ident), &op.annotations)
        .map_err(ParseError::Message)?;
    let stream = http_stream_config(&op.annotations).map_err(ParseError::Message)?;
    validate_stream_shape(&op.ident, stream).map_err(parse_err)?;
    let default_method = if matches!(
        stream.kind,
        Some(HttpStreamKind::Server) | Some(HttpStreamKind::Bidi)
    ) {
        super::HttpMethod::Get
    } else {
        super::HttpMethod::Post
    };
    let (method, mut route_paths) =
        route_from_annotations(&op.annotations, default_method).map_err(parse_err)?;
    validate_stream_method(&op.ident, stream.kind, method).map_err(parse_err)?;
    if route_paths.is_empty() {
        route_paths.push(auto_default_method_path(op, method).map_err(parse_err)?);
    }
    let routes = route_paths
        .iter()
        .map(|path| parse_route_template(path).map_err(parse_err))
        .collect::<ParserResult<Vec<_>>>()?;
    let route_path_names = routes
        .iter()
        .map(|route| route.path_params.iter().cloned().collect::<HashSet<_>>())
        .collect::<Vec<_>>();
    let route_query_names = routes
        .iter()
        .map(|route| route.query_params.iter().cloned().collect::<HashSet<_>>())
        .collect::<Vec<_>>();

    let (request_params, response_params, path_binding_count, query_binding_count) =
        project_params(
            op,
            method,
            stream.kind,
            &route_path_names,
            &route_query_names,
        )
        .map_err(parse_err)?;

    validate_route_bindings(
        &op.ident,
        &routes,
        &path_binding_count,
        &query_binding_count,
    )
    .map_err(parse_err)?;
    validate_request_shape(&op.ident, stream.kind, &request_params).map_err(parse_err)?;
    validate_head_constraints(
        &op.ident,
        method,
        &response_params,
        match &op.ty {
            hir::OpTypeSpec::Void => None,
            hir::OpTypeSpec::TypeSpec(ty) => Some(ty),
        },
    )
    .map_err(parse_err)?;

    let request_body_shape = body_shape(
        &request_params
            .iter()
            .filter(|p| matches!(p.kind, super::HttpParamKind::Body))
            .cloned()
            .collect::<Vec<_>>(),
    );
    let response_body_params = response_params
        .iter()
        .filter(|p| matches!(p.kind, super::HttpParamKind::Body))
        .cloned()
        .collect::<Vec<_>>();
    let response_body_shape = response_body_shape(&op.ty, &response_body_params);

    let request_content_type = request_content_type(
        interface_annotations,
        &op.annotations,
        stream.kind,
        &request_params,
        request_body_shape,
    );
    let response_content_type = response_content_type(
        interface_annotations,
        &op.annotations,
        stream.kind,
        stream.codec,
        response_body_shape,
        &response_body_params,
        match &op.ty {
            hir::OpTypeSpec::Void => None,
            hir::OpTypeSpec::TypeSpec(ty) => Some(ty),
        },
    );

    Ok(HttpOperation {
        name: op.ident.clone(),
        operation_id: operation_id(module_path, interface_name, &op.ident),
        source: HttpOperationSource::Method,
        method,
        routes,
        stream,
        request_content_type,
        response_content_type,
        request_shape: request_shape(&request_params),
        response_shape: response_wrapper_shape(&op.ty, &response_params),
        request_body_shape,
        response_body_shape,
        security: effective_security_with_origin(interface_annotations, &op.annotations)
            .map_err(ParseError::Message)?,
        basic_auth_realm: effective_basic_auth_realm(interface_annotations, &op.annotations),
        deprecated: effective_deprecated(interface_annotations, &op.annotations)
            .map_err(ParseError::Message)?,
        request_params,
        response_params,
        return_type: match &op.ty {
            hir::OpTypeSpec::Void => None,
            hir::OpTypeSpec::TypeSpec(ty) => Some(ty.clone()),
        },
    })
}

fn request_content_type(
    interface_annotations: &[hir::Annotation],
    method_annotations: &[hir::Annotation],
    stream_kind: Option<HttpStreamKind>,
    request_params: &[super::HttpParam],
    body_shape: super::HttpBodyShape,
) -> String {
    if matches!(stream_kind, Some(HttpStreamKind::Client)) {
        return "application/x-ndjson".to_string();
    }
    let body_params = request_params
        .iter()
        .filter(|p| matches!(p.kind, super::HttpParamKind::Body))
        .collect::<Vec<_>>();

    let default = if body_params.is_empty() {
        "application/json"
    } else {
        match body_shape {
            super::HttpBodyShape::Single => {
                if body_params[0].ty.is_composite() {
                    "application/json"
                } else {
                    "text/plain"
                }
            }
            super::HttpBodyShape::SingleFlattened | super::HttpBodyShape::Object => {
                "application/json"
            }
            super::HttpBodyShape::None => "application/json",
        }
    };

    effective_media_type(
        interface_annotations,
        method_annotations,
        "Consumes",
        default,
    )
}

fn response_content_type(
    interface_annotations: &[hir::Annotation],
    method_annotations: &[hir::Annotation],
    stream_kind: Option<HttpStreamKind>,
    stream_codec: super::semantics::HttpStreamCodec,
    body_shape: super::HttpBodyShape,
    response_body_params: &[super::HttpParam],
    return_type: Option<&hir::TypeSpec>,
) -> String {
    if matches!(stream_kind, Some(HttpStreamKind::Server))
        && matches!(stream_codec, super::semantics::HttpStreamCodec::Sse)
    {
        return "text/event-stream".to_string();
    }

    let default = match body_shape {
        super::HttpBodyShape::Single => {
            let is_composite = if let Some(ty) = return_type {
                ty.is_composite()
            } else if let Some(param) = response_body_params.first() {
                param.ty.is_composite()
            } else {
                true
            };
            if is_composite {
                "application/json"
            } else {
                "text/plain"
            }
        }
        super::HttpBodyShape::SingleFlattened | super::HttpBodyShape::Object => "application/json",
        _ => "application/json",
    };

    effective_media_type(
        interface_annotations,
        method_annotations,
        "Produces",
        default,
    )
}

fn request_shape(request_params: &[super::HttpParam]) -> super::HttpRequestShape {
    if request_params.is_empty() {
        super::HttpRequestShape::None
    } else {
        super::HttpRequestShape::Object
    }
}

fn body_shape(body_params: &[super::HttpParam]) -> super::HttpBodyShape {
    if body_params.is_empty() {
        super::HttpBodyShape::None
    } else if body_params.len() == 1 && body_params[0].flatten {
        super::HttpBodyShape::SingleFlattened
    } else if body_params.len() == 1 {
        super::HttpBodyShape::Single
    } else {
        super::HttpBodyShape::Object
    }
}

fn response_wrapper_shape(
    return_ty: &hir::OpTypeSpec,
    response_params: &[super::HttpParam],
) -> super::HttpResponseShape {
    let has_return = !matches!(return_ty, hir::OpTypeSpec::Void);
    let out_param_count = response_params.len();

    match (has_return, out_param_count) {
        (false, 0) => super::HttpResponseShape::None,
        (true, 0) => super::HttpResponseShape::ReturnOnly,
        _ => super::HttpResponseShape::Object,
    }
}

fn response_body_shape(
    return_ty: &hir::OpTypeSpec,
    response_body_params: &[super::HttpParam],
) -> super::HttpBodyShape {
    let has_return = !matches!(return_ty, hir::OpTypeSpec::Void);
    let body_param_count = response_body_params.len();

    match (has_return, body_param_count) {
        (false, 0) => super::HttpBodyShape::None,
        (true, 0) => super::HttpBodyShape::Single,
        (false, 1) => {
            if response_body_params[0].flatten {
                super::HttpBodyShape::SingleFlattened
            } else {
                super::HttpBodyShape::Single
            }
        }
        _ => super::HttpBodyShape::Object,
    }
}
