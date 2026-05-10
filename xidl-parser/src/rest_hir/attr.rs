use crate::hir;
use convert_case::{Case, Casing};

use super::mapping;
use super::route::{attribute_path, default_path, operation_id, readonly_attr_names};
use super::semantics::{
    DeprecatedInfo, HttpSecurityProfile, HttpStreamCodec, HttpStreamConfig, HttpStreamKind,
};
use super::{
    HttpMethod, HttpOperation, HttpOperationMeta, HttpOperationSignature, HttpOperationSource,
    HttpParam, HttpParamKind, HttpRoute, HttpSignatureParam, HttpSignatureParamAnnotation,
    HttpSignatureParamDirection,
};

#[cfg(test)]
mod tests;

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
    let name = format!("get_attribute_{raw_name}");
    let signature = HttpOperationSignature {
        params: Vec::new(),
        return_type: Some(ty.clone()),
    };
    base_attribute_operation(AttributeOperationArgs {
        interface_name,
        module_path,
        name: &name,
        source: HttpOperationSource::AttributeGet,
        method: HttpMethod::Get,
        routes: vec![HttpRoute {
            path: attribute_path(raw_name),
            path_params: Vec::new(),
            query_params: Vec::new(),
        }],
        stream: HttpStreamConfig {
            kind: None,
            codec: HttpStreamCodec::Ndjson,
        },
        security,
        deprecated,
        request_params: Vec::new(),
        response_params: Vec::new(),
        return_type: Some(ty.clone()),
        signature,
    })
}

fn attribute_set_operation(
    interface_name: &str,
    module_path: &[String],
    raw_name: &str,
    ty: &hir::TypeSpec,
    security: Option<HttpSecurityProfile>,
    deprecated: Option<DeprecatedInfo>,
) -> HttpOperation {
    let name = format!("set_attribute_{raw_name}");
    let param_name = raw_name.to_case(Case::Snake);
    let value = HttpParam {
        name: param_name.clone(),
        wire_name: raw_name.to_string(),
        ty: ty.clone(),
        kind: HttpParamKind::Body,
        optional: false,
        flatten: false,
    };
    let signature = HttpOperationSignature {
        params: vec![HttpSignatureParam {
            name: param_name,
            ty: ty.clone(),
            direction: HttpSignatureParamDirection::In,
            is_optional: false,
            is_flatten: false,
            annotations: vec![HttpSignatureParamAnnotation::Body],
        }],
        return_type: None,
    };
    base_attribute_operation(AttributeOperationArgs {
        interface_name,
        module_path,
        name: &name,
        source: HttpOperationSource::AttributeSet,
        method: HttpMethod::Post,
        routes: vec![HttpRoute {
            path: attribute_path(raw_name),
            path_params: Vec::new(),
            query_params: Vec::new(),
        }],
        stream: HttpStreamConfig {
            kind: None,
            codec: HttpStreamCodec::Ndjson,
        },
        security,
        deprecated,
        request_params: vec![value],
        response_params: Vec::new(),
        return_type: None,
        signature,
    })
}

fn attribute_watch_operation(
    interface_name: &str,
    module_path: &[String],
    raw_name: &str,
    ty: &hir::TypeSpec,
    security: Option<HttpSecurityProfile>,
    deprecated: Option<DeprecatedInfo>,
) -> HttpOperation {
    let name = format!("watch_attribute_{raw_name}");
    let signature = HttpOperationSignature {
        params: Vec::new(),
        return_type: Some(ty.clone()),
    };
    base_attribute_operation(AttributeOperationArgs {
        interface_name,
        module_path,
        name: &name,
        source: HttpOperationSource::AttributeWatch,
        method: HttpMethod::Get,
        routes: vec![HttpRoute {
            path: default_path(module_path, interface_name, &name),
            path_params: Vec::new(),
            query_params: Vec::new(),
        }],
        stream: HttpStreamConfig {
            kind: Some(HttpStreamKind::Server),
            codec: HttpStreamCodec::Sse,
        },
        security,
        deprecated,
        request_params: Vec::new(),
        response_params: Vec::new(),
        return_type: Some(ty.clone()),
        signature,
    })
}

fn base_attribute_operation(args: AttributeOperationArgs<'_>) -> HttpOperation {
    let AttributeOperationArgs {
        interface_name,
        module_path,
        name,
        source,
        method,
        routes,
        stream,
        security,
        deprecated,
        request_params,
        response_params,
        return_type,
        signature,
    } = args;

    let request_content_type = "application/json".to_string();
    let response_content_type = "application/json".to_string();

    let http = mapping::build_http_mapping(
        method,
        &stream,
        &request_content_type,
        &response_content_type,
        &request_params,
        &response_params,
        &return_type,
    );

    HttpOperation {
        meta: HttpOperationMeta {
            name: name.to_string(),
            operation_id: operation_id(module_path, interface_name, name),
            source,
            method,
            routes,
            stream,
            security,
            basic_auth_realm: None,
            deprecated,
        },
        signature,
        http,
    }
}

struct AttributeOperationArgs<'a> {
    interface_name: &'a str,
    module_path: &'a [String],
    name: &'a str,
    source: HttpOperationSource,
    method: HttpMethod,
    routes: Vec<HttpRoute>,
    stream: HttpStreamConfig,
    security: Option<HttpSecurityProfile>,
    deprecated: Option<DeprecatedInfo>,
    request_params: Vec<HttpParam>,
    response_params: Vec<HttpParam>,
    return_type: Option<hir::TypeSpec>,
    signature: HttpOperationSignature,
}
