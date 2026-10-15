//! Operation projection: one interface operation becomes its typed HTTP
//! operation with method, routes, stream, security and response shapes.

use crate::error::{ParseError, ParserResult};
use crate::hir;

use std::collections::HashSet;

use super::super::model::{
    HttpException, HttpExceptionRef, HttpOperation, HttpOperationMeta, HttpOperationSource,
    HttpRepresentation, HttpUnion,
};
use super::super::project_params::project_params;
use super::super::route::{
    auto_default_method_path, operation_id, parse_route_template, route_from_annotations,
};
use super::super::semantics::{
    HttpStreamKind, UpgradeMode, classify_upgrade_protocol, effective_cors, effective_media_type,
    effective_security_with_origin, has_annotation, http_stream_config, parse_upgrade_protocol,
    parse_websocket_config, validate_http_annotations,
};
use super::super::validate::{
    effective_basic_auth_realm, effective_deprecated, validate_head_constraints,
    validate_request_shape, validate_route_bindings, validate_stream_method, validate_stream_shape,
    validate_upgrade_constraints, validate_websocket_stream_items,
};
use super::super::{HttpMethod, mapping};
use super::parse_err;

pub(super) fn project_operation(
    interface_name: &str,
    module_path: &[String],
    interface_annotations: &[hir::Annotation],
    exceptions: &[HttpException],
    http_unions: &[HttpUnion],
    op: &hir::OpDcl,
) -> ParserResult<HttpOperation> {
    validate_http_annotations(&format!("operation '{}'", op.ident), &op.annotations)
        .map_err(ParseError::Message)?;
    let mut stream = http_stream_config(&op.annotations).map_err(ParseError::Message)?;
    validate_stream_shape(&op.ident, stream).map_err(parse_err)?;
    let has_upgrade = has_annotation(&op.annotations, "upgrade");
    let has_stream_annotation = has_annotation(&op.annotations, "server_stream")
        || has_annotation(&op.annotations, "client_stream")
        || has_annotation(&op.annotations, "bidi_stream");
    if has_upgrade && has_stream_annotation {
        return Err(parse_err(
            "@upgrade cannot be combined with @server_stream/@client_stream/@bidi_stream"
                .to_string(),
        ));
    }

    let upgrade_protocol = parse_upgrade_protocol(&op.annotations).map_err(parse_err)?;
    let upgrade_mode = upgrade_protocol
        .as_deref()
        .map(classify_upgrade_protocol)
        .transpose()
        .map_err(parse_err)?;
    let websocket = match upgrade_mode {
        Some(mode) => parse_websocket_config(&op.annotations, mode).map_err(parse_err)?,
        None => None,
    };
    // WebSocket mode is projected as a bidirectional stream plus WS config.
    if upgrade_mode == Some(UpgradeMode::WebSocket) {
        stream.kind = Some(HttpStreamKind::Bidi);
    }

    let default_method = if matches!(
        stream.kind,
        Some(HttpStreamKind::Server) | Some(HttpStreamKind::Bidi)
    ) || has_upgrade
    {
        HttpMethod::Get
    } else {
        HttpMethod::Post
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
    let allow_handshake_params = upgrade_mode == Some(UpgradeMode::WebSocket);
    validate_request_shape(
        &op.ident,
        stream.kind,
        &request_params,
        allow_handshake_params,
    )
    .map_err(parse_err)?;
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

    validate_upgrade_constraints(
        &op.ident,
        upgrade_mode,
        method,
        &request_params,
        match &op.ty {
            hir::OpTypeSpec::Void => None,
            hir::OpTypeSpec::TypeSpec(ty) => Some(ty),
        },
    )
    .map_err(parse_err)?;
    if upgrade_mode == Some(UpgradeMode::WebSocket) {
        validate_websocket_stream_items(&op.ident, &request_params, &response_params)
            .map_err(parse_err)?;
    }

    let request_content_type =
        effective_media_type(interface_annotations, &op.annotations, "request");
    let response_content_type =
        effective_media_type(interface_annotations, &op.annotations, "response");
    let cors =
        effective_cors(interface_annotations, &op.annotations).map_err(ParseError::Message)?;
    let security = effective_security_with_origin(interface_annotations, &op.annotations)
        .map_err(ParseError::Message)?;
    let basic_auth_realm = effective_basic_auth_realm(interface_annotations, &op.annotations);
    let deprecated = effective_deprecated(interface_annotations, &op.annotations)
        .map_err(ParseError::Message)?;

    let return_type = match &op.ty {
        hir::OpTypeSpec::Void => None,
        hir::OpTypeSpec::TypeSpec(ty) => Some(ty.clone()),
    };

    let signature = mapping::build_operation_signature(op);
    let http = mapping::build_http_mapping(
        method,
        &stream,
        request_content_type.as_deref(),
        response_content_type.as_deref(),
        &request_params,
        &response_params,
        &return_type,
    );
    if op.raises.is_some() && (stream.kind.is_some() || has_upgrade) {
        return Err(ParseError::Message(format!(
            "operation '{}': raises(...) is only supported on non-stream, non-upgrade operations",
            op.ident
        )));
    }
    let raises = HttpExceptionRef::for_operation(op, module_path, exceptions)?;
    let mut operation = HttpOperation {
        meta: HttpOperationMeta {
            name: op.ident.clone(),
            operation_id: operation_id(module_path, interface_name, &op.ident),
            source: HttpOperationSource::Method,
            method,
            routes,
            stream,
            cors,
            security,
            basic_auth_realm,
            deprecated,
            upgrade_protocol,
            upgrade_mode,
            websocket,
            raises,
        },
        signature,
        http,
    };
    operation.http.response.representations =
        HttpRepresentation::for_operation(&operation, module_path, http_unions, has_upgrade)?;
    Ok(operation)
}
