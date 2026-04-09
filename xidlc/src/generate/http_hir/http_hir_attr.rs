use convert_case::{Case, Casing};
use xidl_parser::hir;

use super::http_hir_route::{attribute_path, default_path, operation_id, readonly_attr_names};
use super::semantics::{
    DeprecatedInfo, HttpSecurityProfile, HttpStreamCodec, HttpStreamConfig, HttpStreamKind,
};
use super::{
    HttpMethod, HttpOperation, HttpOperationSource, HttpParam, HttpParamDirection, HttpParamSource,
    HttpRoute,
};

pub(super) fn project_attribute(
    interface_name: &str,
    module_path: &[String],
    attr: &hir::AttrDcl,
    security: Option<HttpSecurityProfile>,
    deprecated: Option<DeprecatedInfo>,
    emit_watch: bool,
) -> Vec<HttpOperation> {
    let mut operations = Vec::new();
    match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => {
            for raw_name in readonly_attr_names(spec) {
                operations.push(attribute_get_operation(
                    interface_name,
                    module_path,
                    &raw_name,
                    &spec.ty,
                    security.clone(),
                    deprecated.clone(),
                ));
                if emit_watch {
                    operations.push(attribute_watch_operation(
                        interface_name,
                        module_path,
                        &raw_name,
                        &spec.ty,
                        security.clone(),
                        deprecated.clone(),
                    ));
                }
            }
        }
        hir::AttrDclInner::AttrSpec(spec) => match &spec.declarator {
            hir::AttrDeclarator::SimpleDeclarator(values) => {
                for decl in values {
                    operations.extend(attribute_rw_operations(
                        interface_name,
                        module_path,
                        &decl.0,
                        &spec.ty,
                        security.clone(),
                        deprecated.clone(),
                        emit_watch,
                    ));
                }
            }
            hir::AttrDeclarator::WithRaises { declarator, .. } => {
                operations.extend(attribute_rw_operations(
                    interface_name,
                    module_path,
                    &declarator.0,
                    &spec.ty,
                    security,
                    deprecated,
                    emit_watch,
                ));
            }
        },
    }
    operations
}

fn attribute_rw_operations(
    interface_name: &str,
    module_path: &[String],
    raw_name: &str,
    ty: &hir::TypeSpec,
    security: Option<HttpSecurityProfile>,
    deprecated: Option<DeprecatedInfo>,
    emit_watch: bool,
) -> Vec<HttpOperation> {
    let mut ops = vec![
        attribute_get_operation(
            interface_name,
            module_path,
            raw_name,
            ty,
            security.clone(),
            deprecated.clone(),
        ),
        attribute_set_operation(
            interface_name,
            module_path,
            raw_name,
            ty,
            security.clone(),
            deprecated.clone(),
        ),
    ];
    if emit_watch {
        ops.push(attribute_watch_operation(
            interface_name,
            module_path,
            raw_name,
            ty,
            security,
            deprecated,
        ));
    }
    ops
}

fn attribute_get_operation(
    interface_name: &str,
    module_path: &[String],
    raw_name: &str,
    ty: &hir::TypeSpec,
    security: Option<HttpSecurityProfile>,
    deprecated: Option<DeprecatedInfo>,
) -> HttpOperation {
    base_attribute_operation(
        interface_name,
        module_path,
        &format!("get_attribute_{raw_name}"),
        HttpOperationSource::AttributeGet,
        HttpMethod::Get,
        vec![HttpRoute {
            path: attribute_path(raw_name),
            path_params: Vec::new(),
            query_params: Vec::new(),
        }],
        HttpStreamConfig {
            kind: None,
            codec: HttpStreamCodec::Ndjson,
        },
        security,
        deprecated,
        Vec::new(),
        Vec::new(),
        Some(ty.clone()),
    )
}

fn attribute_set_operation(
    interface_name: &str,
    module_path: &[String],
    raw_name: &str,
    ty: &hir::TypeSpec,
    security: Option<HttpSecurityProfile>,
    deprecated: Option<DeprecatedInfo>,
) -> HttpOperation {
    let value = HttpParam {
        name: raw_name.to_case(Case::Snake),
        wire_name: raw_name.to_string(),
        ty: ty.clone(),
        direction: HttpParamDirection::In,
        source: HttpParamSource::Body,
        optional: false,
        flatten: false,
    };
    base_attribute_operation(
        interface_name,
        module_path,
        &format!("set_attribute_{raw_name}"),
        HttpOperationSource::AttributeSet,
        HttpMethod::Post,
        vec![HttpRoute {
            path: attribute_path(raw_name),
            path_params: Vec::new(),
            query_params: Vec::new(),
        }],
        HttpStreamConfig {
            kind: None,
            codec: HttpStreamCodec::Ndjson,
        },
        security,
        deprecated,
        vec![value.clone()],
        vec![value],
        None,
    )
}

fn attribute_watch_operation(
    interface_name: &str,
    module_path: &[String],
    raw_name: &str,
    ty: &hir::TypeSpec,
    security: Option<HttpSecurityProfile>,
    deprecated: Option<DeprecatedInfo>,
) -> HttpOperation {
    base_attribute_operation(
        interface_name,
        module_path,
        &format!("watch_attribute_{raw_name}"),
        HttpOperationSource::AttributeWatch,
        HttpMethod::Get,
        vec![HttpRoute {
            path: default_path(
                module_path,
                interface_name,
                &format!("watch_attribute_{raw_name}"),
            ),
            path_params: Vec::new(),
            query_params: Vec::new(),
        }],
        HttpStreamConfig {
            kind: Some(HttpStreamKind::Server),
            codec: HttpStreamCodec::Sse,
        },
        security,
        deprecated,
        Vec::new(),
        Vec::new(),
        Some(ty.clone()),
    )
}

fn base_attribute_operation(
    interface_name: &str,
    module_path: &[String],
    name: &str,
    source: HttpOperationSource,
    method: HttpMethod,
    routes: Vec<HttpRoute>,
    stream: HttpStreamConfig,
    security: Option<HttpSecurityProfile>,
    deprecated: Option<DeprecatedInfo>,
    request_params: Vec<HttpParam>,
    request_body_params: Vec<HttpParam>,
    return_type: Option<hir::TypeSpec>,
) -> HttpOperation {
    HttpOperation {
        name: name.to_string(),
        operation_id: operation_id(module_path, interface_name, name),
        source,
        method,
        routes,
        stream,
        request_content_type: "application/json".to_string(),
        response_content_type: "application/json".to_string(),
        security,
        basic_auth_realm: None,
        deprecated,
        request_params,
        request_path_params: Vec::new(),
        request_query_params: Vec::new(),
        request_header_params: Vec::new(),
        request_cookie_params: Vec::new(),
        request_body_params,
        response_params: Vec::new(),
        response_header_params: Vec::new(),
        response_cookie_params: Vec::new(),
        response_body_params: Vec::new(),
        return_type,
    }
}
