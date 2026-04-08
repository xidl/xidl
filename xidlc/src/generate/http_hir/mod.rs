use crate::error::{IdlcError, IdlcResult};
pub mod semantics;

use self::semantics::{
    DeprecatedInfo, HttpSecurityProfile, HttpStreamCodec, HttpStreamConfig, HttpStreamKind,
    annotation_name, annotation_params, deprecated_info, effective_media_type,
    effective_security_with_origin, has_annotation, has_optional_annotation, http_stream_config,
    normalize_annotation_params, validate_http_annotations,
};
use convert_case::{Case, Casing};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use xidl_parser::hir;

use crate::jsonrpc::{Artifact, ArtifactFile, ArtifactHttpHir};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpParamDirection {
    In,
    Out,
    InOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpParamSource {
    Path,
    Query,
    Header,
    Cookie,
    Body,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpOperationSource {
    Method,
    AttributeGet,
    AttributeSet,
    AttributeWatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpRoute {
    pub path: String,
    pub path_params: Vec<String>,
    pub query_params: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpParam {
    pub name: String,
    pub wire_name: String,
    pub ty: hir::TypeSpec,
    pub direction: HttpParamDirection,
    pub source: HttpParamSource,
    pub optional: bool,
    pub flatten: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpDocumentServer {
    pub base_url: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HttpDocumentMetadata {
    pub package: Option<String>,
    pub version: Option<String>,
    pub servers: Vec<HttpDocumentServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpOperation {
    pub name: String,
    pub operation_id: String,
    pub source: HttpOperationSource,
    pub method: HttpMethod,
    pub routes: Vec<HttpRoute>,
    pub stream: HttpStreamConfig,
    pub request_content_type: String,
    pub response_content_type: String,
    pub security: Option<HttpSecurityProfile>,
    pub basic_auth_realm: Option<String>,
    pub deprecated: Option<DeprecatedInfo>,
    pub request_params: Vec<HttpParam>,
    pub request_path_params: Vec<HttpParam>,
    pub request_query_params: Vec<HttpParam>,
    pub request_header_params: Vec<HttpParam>,
    pub request_cookie_params: Vec<HttpParam>,
    pub request_body_params: Vec<HttpParam>,
    pub response_params: Vec<HttpParam>,
    pub response_header_params: Vec<HttpParam>,
    pub response_cookie_params: Vec<HttpParam>,
    pub response_body_params: Vec<HttpParam>,
    pub return_type: Option<hir::TypeSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpInterface {
    pub name: String,
    pub module_path: Vec<String>,
    pub operations: Vec<HttpOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HttpHirDocument {
    pub document: HttpDocumentMetadata,
    pub interfaces: Vec<HttpInterface>,
}

impl HttpHirDocument {
    pub fn from_props(props: &hir::ParserProperties) -> IdlcResult<Self> {
        let value = props
            .get("http_hir")
            .cloned()
            .ok_or_else(|| IdlcError::rpc("missing http_hir properties".to_string()))?;
        serde_json::from_value(value).map_err(|err| IdlcError::rpc(err.to_string()))
    }

    pub fn find_interface(&self, module_path: &[String], name: &str) -> Option<&HttpInterface> {
        self.interfaces
            .iter()
            .find(|interface| interface.name == name && interface.module_path == module_path)
    }
}

pub(crate) struct HttpHirCodegen;

#[async_trait::async_trait]
impl crate::jsonrpc::Codegen for HttpHirCodegen {
    async fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("*".to_string())
    }

    async fn get_properties(&self) -> Result<hir::ParserProperties, xidl_jsonrpc::Error> {
        Ok(HashMap::new())
    }

    async fn generate(
        &self,
        hir: hir::Specification,
        path: String,
        props: hir::ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        let target_lang: String = serde_json::from_value(
            props
                .get("target_lang")
                .cloned()
                .unwrap_or_else(|| serde_json::Value::String("http-hir".to_string())),
        )
        .map_err(|err| xidl_jsonrpc::Error::invalid_params(err.to_string()))?;
        let http_hir = project(&hir).map_err(|err| xidl_jsonrpc::Error::Rpc {
            code: xidl_jsonrpc::ErrorCode::ServerError,
            message: err.to_string(),
            data: None,
        })?;

        if target_lang == "http-hir" {
            let content = serde_json::to_string_pretty(&http_hir)?;
            Ok(vec![Artifact::new_file(ArtifactFile {
                path: path.replace(".idl", ".http_hir.json"),
                content,
            })])
        } else {
            Ok(vec![Artifact::new_http_hir(ArtifactHttpHir {
                lang: target_lang,
                hir,
                http_hir,
                props,
            })])
        }
    }
}

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
            } if !base_url.is_empty() => self.document.servers.push(HttpDocumentServer {
                base_url: base_url.clone(),
                description: description.clone(),
            }),
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
                hir::Export::AttrDcl(attr) => operations.extend(project_attribute(
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
        HttpMethod::Get
    } else {
        HttpMethod::Post
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

    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    let mut request_params = Vec::new();
    let mut request_path_params = Vec::new();
    let mut request_query_params = Vec::new();
    let mut request_header_params = Vec::new();
    let mut request_cookie_params = Vec::new();
    let mut request_body_params = Vec::new();
    let mut response_params = Vec::new();
    let mut response_header_params = Vec::new();
    let mut response_cookie_params = Vec::new();
    let mut response_body_params = Vec::new();
    let mut path_binding_count = HashMap::<String, usize>::new();
    let mut query_binding_count = HashMap::<String, usize>::new();

    for param in params {
        let direction = param_direction(param.attr.as_ref());
        let binding = explicit_param_binding(param)?;
        let default_source = if matches!(stream.kind, Some(HttpStreamKind::Bidi)) {
            HttpParamSource::Body
        } else {
            default_param_source(method)
        };
        let inferred_source =
            match direction {
                HttpParamDirection::Out | HttpParamDirection::InOut => binding
                    .as_ref()
                    .map(|value| value.source)
                    .unwrap_or(HttpParamSource::Body),
                HttpParamDirection::In => binding
                    .as_ref()
                    .map(|value| value.source)
                    .unwrap_or_else(|| {
                        if route_path_names
                            .iter()
                            .all(|set| set.contains(&param.declarator.0))
                        {
                            HttpParamSource::Path
                        } else if route_query_names
                            .iter()
                            .any(|set| set.contains(&param.declarator.0))
                        {
                            HttpParamSource::Query
                        } else {
                            default_source
                        }
                    }),
            };
        let projected = HttpParam {
            name: param.declarator.0.clone(),
            wire_name: binding
                .as_ref()
                .map(|value| value.bound_name.clone())
                .unwrap_or_else(|| param.declarator.0.clone()),
            ty: param.ty.clone(),
            direction,
            source: inferred_source,
            optional: has_optional_annotation(&param.annotations),
            flatten: has_annotation(&param.annotations, "flatten"),
        };
        let projected_source = projected.source;
        validate_projected_param(
            &op.ident,
            &projected,
            direction,
            &route_path_names,
            &route_query_names,
        )?;
        match direction {
            HttpParamDirection::In | HttpParamDirection::InOut => {
                request_params.push(projected.clone());
                match projected_source {
                    HttpParamSource::Path => {
                        *path_binding_count
                            .entry(projected.wire_name.clone())
                            .or_insert(0) += 1;
                        request_path_params.push(projected.clone());
                    }
                    HttpParamSource::Query => {
                        *query_binding_count
                            .entry(projected.wire_name.clone())
                            .or_insert(0) += 1;
                        request_query_params.push(projected.clone());
                    }
                    HttpParamSource::Header => request_header_params.push(projected.clone()),
                    HttpParamSource::Cookie => request_cookie_params.push(projected.clone()),
                    HttpParamSource::Body => request_body_params.push(projected.clone()),
                }
            }
            HttpParamDirection::Out => {}
        }
        match direction {
            HttpParamDirection::Out | HttpParamDirection::InOut => {
                response_params.push(projected.clone());
                match projected_source {
                    HttpParamSource::Header => response_header_params.push(projected),
                    HttpParamSource::Cookie => response_cookie_params.push(projected),
                    _ => response_body_params.push(projected),
                }
            }
            HttpParamDirection::In => {}
        }
    }

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

fn project_attribute(
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
    let emit_watch = has_annotation(&attr.annotations, "server_stream");
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
                    operations.push(attribute_get_operation(
                        interface_name,
                        module_path,
                        &decl.0,
                        &spec.ty,
                        security.clone(),
                        deprecated.clone(),
                    ));
                    operations.push(attribute_set_operation(
                        interface_name,
                        module_path,
                        &decl.0,
                        &spec.ty,
                        security.clone(),
                        deprecated.clone(),
                    ));
                    if emit_watch {
                        operations.push(attribute_watch_operation(
                            interface_name,
                            module_path,
                            &decl.0,
                            &spec.ty,
                            security.clone(),
                            deprecated.clone(),
                        ));
                    }
                }
            }
            hir::AttrDeclarator::WithRaises { declarator, .. } => {
                operations.push(attribute_get_operation(
                    interface_name,
                    module_path,
                    &declarator.0,
                    &spec.ty,
                    security.clone(),
                    deprecated.clone(),
                ));
                operations.push(attribute_set_operation(
                    interface_name,
                    module_path,
                    &declarator.0,
                    &spec.ty,
                    security.clone(),
                    deprecated.clone(),
                ));
                if emit_watch {
                    operations.push(attribute_watch_operation(
                        interface_name,
                        module_path,
                        &declarator.0,
                        &spec.ty,
                        security,
                        deprecated,
                    ));
                }
            }
        },
    }

    Ok(operations)
}

fn attribute_get_operation(
    interface_name: &str,
    module_path: &[String],
    raw_name: &str,
    ty: &hir::TypeSpec,
    security: Option<HttpSecurityProfile>,
    deprecated: Option<DeprecatedInfo>,
) -> HttpOperation {
    HttpOperation {
        name: format!("get_attribute_{raw_name}"),
        operation_id: operation_id(
            module_path,
            interface_name,
            &format!("get_attribute_{raw_name}"),
        ),
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
        request_content_type: "application/json".to_string(),
        response_content_type: "application/json".to_string(),
        security,
        basic_auth_realm: None,
        deprecated,
        request_params: Vec::new(),
        request_path_params: Vec::new(),
        request_query_params: Vec::new(),
        request_header_params: Vec::new(),
        request_cookie_params: Vec::new(),
        request_body_params: Vec::new(),
        response_params: Vec::new(),
        response_header_params: Vec::new(),
        response_cookie_params: Vec::new(),
        response_body_params: Vec::new(),
        return_type: Some(ty.clone()),
    }
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
    HttpOperation {
        name: format!("set_attribute_{raw_name}"),
        operation_id: operation_id(
            module_path,
            interface_name,
            &format!("set_attribute_{raw_name}"),
        ),
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
        request_content_type: "application/json".to_string(),
        response_content_type: "application/json".to_string(),
        security,
        basic_auth_realm: None,
        deprecated,
        request_params: vec![value.clone()],
        request_path_params: Vec::new(),
        request_query_params: Vec::new(),
        request_header_params: Vec::new(),
        request_cookie_params: Vec::new(),
        request_body_params: vec![value],
        response_params: Vec::new(),
        response_header_params: Vec::new(),
        response_cookie_params: Vec::new(),
        response_body_params: Vec::new(),
        return_type: None,
    }
}

fn attribute_watch_operation(
    interface_name: &str,
    module_path: &[String],
    raw_name: &str,
    ty: &hir::TypeSpec,
    security: Option<HttpSecurityProfile>,
    deprecated: Option<DeprecatedInfo>,
) -> HttpOperation {
    HttpOperation {
        name: format!("watch_attribute_{raw_name}"),
        operation_id: operation_id(
            module_path,
            interface_name,
            &format!("watch_attribute_{raw_name}"),
        ),
        source: HttpOperationSource::AttributeWatch,
        method: HttpMethod::Get,
        routes: vec![HttpRoute {
            path: default_path(
                module_path,
                interface_name,
                &format!("watch_attribute_{raw_name}"),
            ),
            path_params: Vec::new(),
            query_params: Vec::new(),
        }],
        stream: HttpStreamConfig {
            kind: Some(HttpStreamKind::Server),
            codec: HttpStreamCodec::Sse,
        },
        request_content_type: "application/json".to_string(),
        response_content_type: "application/json".to_string(),
        security,
        basic_auth_realm: None,
        deprecated,
        request_params: Vec::new(),
        request_path_params: Vec::new(),
        request_query_params: Vec::new(),
        request_header_params: Vec::new(),
        request_cookie_params: Vec::new(),
        request_body_params: Vec::new(),
        response_params: Vec::new(),
        response_header_params: Vec::new(),
        response_cookie_params: Vec::new(),
        response_body_params: Vec::new(),
        return_type: Some(ty.clone()),
    }
}

#[derive(Clone)]
struct SourceBinding {
    source: HttpParamSource,
    bound_name: String,
}

fn explicit_param_binding(param: &hir::ParamDcl) -> IdlcResult<Option<SourceBinding>> {
    let mut found = None;
    for annotation in &param.annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        let current = if name.eq_ignore_ascii_case("path") {
            Some(HttpParamSource::Path)
        } else if name.eq_ignore_ascii_case("query") {
            Some(HttpParamSource::Query)
        } else if name.eq_ignore_ascii_case("body") {
            Some(HttpParamSource::Body)
        } else if name.eq_ignore_ascii_case("header") {
            Some(HttpParamSource::Header)
        } else if name.eq_ignore_ascii_case("cookie") {
            Some(HttpParamSource::Cookie)
        } else {
            None
        };
        let Some(current) = current else {
            continue;
        };
        let bound_name = annotation_params(annotation)
            .map(normalize_annotation_params)
            .and_then(|params| params.get("value").cloned())
            .unwrap_or_else(|| param.declarator.0.clone());
        if matches!(current, HttpParamSource::Header) {
            validate_header_name(&bound_name, &param.declarator.0)?;
        }
        if matches!(current, HttpParamSource::Cookie) {
            validate_cookie_name(&bound_name, &param.declarator.0)?;
        }
        match &found {
            None => {
                found = Some(SourceBinding {
                    source: current,
                    bound_name,
                })
            }
            Some(previous) if previous.source == current && previous.bound_name == bound_name => {}
            Some(_) => {
                return Err(IdlcError::rpc(format!(
                    "parameter '{}' has conflicting source annotations (@path/@query/@body/@header/@cookie)",
                    param.declarator.0
                )));
            }
        }
    }
    Ok(found)
}

fn route_from_annotations(
    annotations: &[hir::Annotation],
    default_method: HttpMethod,
) -> IdlcResult<(HttpMethod, Vec<String>)> {
    let mut method = None;
    let mut paths = Vec::new();
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if let Some(current) = method_from_annotation(annotation) {
            if let Some(previous) = method {
                if previous != current {
                    return Err(IdlcError::rpc(
                        "more than one HTTP verb annotation is not allowed on a method",
                    ));
                }
            }
            method = Some(current);
            if let Some(path) = annotation_params(annotation)
                .map(normalize_annotation_params)
                .and_then(|params| params.get("path").cloned())
            {
                paths.push(normalize_path(&path));
            }
            continue;
        }
        if name.eq_ignore_ascii_case("path") {
            if let Some(path) = annotation_params(annotation)
                .map(normalize_annotation_params)
                .and_then(|params| {
                    params
                        .get("value")
                        .cloned()
                        .or_else(|| params.get("path").cloned())
                })
            {
                paths.push(normalize_path(&path));
            }
        }
    }
    let mut dedup = BTreeSet::new();
    paths.retain(|path| dedup.insert(path.clone()));
    Ok((method.unwrap_or(default_method), paths))
}

fn method_from_annotation(annotation: &hir::Annotation) -> Option<HttpMethod> {
    let name = annotation_name(annotation)?;
    match name.to_ascii_lowercase().as_str() {
        "get" => Some(HttpMethod::Get),
        "post" => Some(HttpMethod::Post),
        "put" => Some(HttpMethod::Put),
        "patch" => Some(HttpMethod::Patch),
        "delete" => Some(HttpMethod::Delete),
        "head" => Some(HttpMethod::Head),
        "options" => Some(HttpMethod::Options),
        _ => None,
    }
}

fn auto_default_method_path(op: &hir::OpDcl, method: HttpMethod) -> IdlcResult<String> {
    let mut path = normalize_path(&op.ident);
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    for param in params {
        if matches!(
            param_direction(param.attr.as_ref()),
            HttpParamDirection::Out
        ) {
            continue;
        }
        let binding = explicit_param_binding(param)?;
        let source = binding
            .as_ref()
            .map(|value| value.source)
            .unwrap_or(default_param_source(method));
        let bound_name = binding
            .as_ref()
            .map(|value| value.bound_name.clone())
            .unwrap_or_else(|| param.declarator.0.clone());
        if matches!(source, HttpParamSource::Path) {
            path.push('/');
            path.push('{');
            path.push_str(&bound_name);
            path.push('}');
        }
    }
    Ok(path)
}

fn parse_route_template(path: &str) -> IdlcResult<HttpRoute> {
    let (path, query_params) = split_query_template(path)?;
    validate_route_template(&path)?;
    let path = normalize_path(&path);
    let mut path_params = parse_path_params(&path).into_iter().collect::<Vec<_>>();
    let mut query_params = query_params.into_iter().collect::<Vec<_>>();
    path_params.sort();
    query_params.sort();
    Ok(HttpRoute {
        path,
        path_params,
        query_params,
    })
}

fn split_query_template(path: &str) -> IdlcResult<(String, HashSet<String>)> {
    let mut query_params = HashSet::new();
    if let Some(pos) = path.find("{?") {
        if !path.ends_with('}') {
            return Err(IdlcError::rpc(format!(
                "query template must terminate with '}}' in route '{path}'"
            )));
        }
        let tail = &path[pos + 2..path.len() - 1];
        if tail.trim().is_empty() {
            return Err(IdlcError::rpc(format!(
                "query template must include at least one variable in route '{path}'"
            )));
        }
        for name in tail.split(',').map(str::trim) {
            if name.is_empty() {
                return Err(IdlcError::rpc(format!(
                    "query template contains empty variable name in route '{path}'"
                )));
            }
            query_params.insert(name.to_string());
        }
        Ok((path[..pos].to_string(), query_params))
    } else {
        Ok((path.to_string(), query_params))
    }
}

fn validate_route_template(path: &str) -> IdlcResult<()> {
    let mut start = 0usize;
    let mut catch_all_count = 0usize;
    while let Some(open_rel) = path[start..].find('{') {
        let open = start + open_rel;
        let close = path[open + 1..]
            .find('}')
            .map(|value| open + 1 + value)
            .ok_or_else(|| {
                IdlcError::rpc(format!("route template has unmatched '{{' in '{path}'"))
            })?;
        let token = &path[open + 1..close];
        let is_catch_all = token.starts_with('*');
        let name = token.strip_prefix('*').unwrap_or(token);
        if name.is_empty() {
            return Err(IdlcError::rpc(format!(
                "route template has empty path variable in '{path}'"
            )));
        }
        if is_catch_all {
            catch_all_count += 1;
            if catch_all_count > 1 {
                return Err(IdlcError::rpc(format!(
                    "route template contains more than one catch-all variable: '{path}'"
                )));
            }
            if close + 1 != path.len() {
                return Err(IdlcError::rpc(format!(
                    "catch-all variable must be at the end of route template: '{path}'"
                )));
            }
        }
        start = close + 1;
    }
    Ok(())
}

fn parse_path_params(path: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut in_param = false;
    let mut current = String::new();
    for ch in path.chars() {
        match ch {
            '{' if !in_param => {
                in_param = true;
                current.clear();
            }
            '}' if in_param => {
                if !current.is_empty() {
                    out.insert(current.trim_start_matches('*').to_string());
                }
                in_param = false;
            }
            _ if in_param => current.push(ch),
            _ => {}
        }
    }
    out
}

fn normalize_path(path: &str) -> String {
    let path = path.trim();
    let with_leading = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let mut out = String::new();
    let mut prev_slash = false;
    for ch in with_leading.chars() {
        if ch == '/' {
            if !prev_slash {
                out.push(ch);
            }
            prev_slash = true;
        } else {
            prev_slash = false;
            out.push(ch);
        }
    }
    if out.len() > 1 && out.ends_with('/') {
        out.pop();
    }
    if out.is_empty() { "/".to_string() } else { out }
}

fn default_param_source(method: HttpMethod) -> HttpParamSource {
    match method {
        HttpMethod::Get | HttpMethod::Delete | HttpMethod::Head | HttpMethod::Options => {
            HttpParamSource::Query
        }
        HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch => HttpParamSource::Body,
    }
}

fn param_direction(attr: Option<&hir::ParamAttribute>) -> HttpParamDirection {
    match attr.map(|value| value.0.as_str()) {
        Some("out") => HttpParamDirection::Out,
        Some("inout") => HttpParamDirection::InOut,
        _ => HttpParamDirection::In,
    }
}

fn effective_deprecated(
    interface_annotations: &[hir::Annotation],
    item_annotations: &[hir::Annotation],
) -> Result<Option<DeprecatedInfo>, String> {
    deprecated_info(item_annotations).and_then(|value| {
        if value.is_some() {
            Ok(value)
        } else {
            deprecated_info(interface_annotations)
        }
    })
}

fn effective_basic_auth_realm(
    interface_annotations: &[hir::Annotation],
    item_annotations: &[hir::Annotation],
) -> Option<String> {
    find_basic_auth_realm(item_annotations).or_else(|| find_basic_auth_realm(interface_annotations))
}

fn find_basic_auth_realm(annotations: &[hir::Annotation]) -> Option<String> {
    annotations.iter().find_map(|annotation| {
        let name = annotation_name(annotation)?;
        if !name.eq_ignore_ascii_case("http_basic") {
            return None;
        }
        annotation_params(annotation)
            .map(normalize_annotation_params)
            .and_then(|params| params.get("realm").cloned())
            .filter(|value| !value.is_empty())
    })
}

fn validate_header_name(bound_name: &str, param_name: &str) -> IdlcResult<()> {
    if bound_name.is_empty() {
        return Err(IdlcError::rpc(format!(
            "parameter '{}' has empty @header name",
            param_name
        )));
    }
    if bound_name.starts_with(':') {
        return Err(IdlcError::rpc(format!(
            "parameter '{}' uses reserved pseudo-header name '{}'",
            param_name, bound_name
        )));
    }
    Ok(())
}

fn validate_cookie_name(bound_name: &str, param_name: &str) -> IdlcResult<()> {
    if bound_name.is_empty() {
        return Err(IdlcError::rpc(format!(
            "parameter '{}' has empty @cookie name",
            param_name
        )));
    }
    if bound_name
        .chars()
        .any(|ch| ch.is_ascii_whitespace() || ch == ';' || ch == '=')
    {
        return Err(IdlcError::rpc(format!(
            "parameter '{}' has invalid @cookie name '{}'",
            param_name, bound_name
        )));
    }
    Ok(())
}

fn validate_stream_shape(op_name: &str, stream: HttpStreamConfig) -> IdlcResult<()> {
    if matches!(
        stream.kind,
        Some(HttpStreamKind::Client | HttpStreamKind::Bidi)
    ) && stream.codec == HttpStreamCodec::Sse
    {
        return Err(IdlcError::rpc(format!(
            "@stream_codec(\"sse\") requires @server_stream on method '{}'",
            op_name
        )));
    }
    Ok(())
}

fn validate_stream_method(
    op_name: &str,
    stream_kind: Option<HttpStreamKind>,
    method: HttpMethod,
) -> IdlcResult<()> {
    let expected = match stream_kind {
        Some(HttpStreamKind::Server) => Some((method, HttpMethod::Get, "@server_stream")),
        Some(HttpStreamKind::Client) => Some((method, HttpMethod::Post, "@client_stream")),
        Some(HttpStreamKind::Bidi) => Some((method, HttpMethod::Get, "@bidi_stream")),
        None => None,
    };
    if let Some((actual, required, label)) = expected {
        if actual != required {
            return Err(IdlcError::rpc(format!(
                "{label} method '{}' must use {}",
                op_name,
                http_method_name(required)
            )));
        }
    }
    Ok(())
}

fn validate_projected_param(
    op_name: &str,
    param: &HttpParam,
    direction: HttpParamDirection,
    route_path_names: &[HashSet<String>],
    _route_query_names: &[HashSet<String>],
) -> IdlcResult<()> {
    if matches!(direction, HttpParamDirection::Out) && param.flatten {
        return Err(IdlcError::rpc(format!(
            "@flatten can only be applied to request-side body parameter '{}' of method '{}'",
            param.name, op_name
        )));
    }
    match param.source {
        HttpParamSource::Path => {
            if route_path_names
                .iter()
                .any(|set| set.contains(&param.wire_name))
            {
                if param.optional {
                    return Err(IdlcError::rpc(format!(
                        "@optional cannot be applied to path parameter '{}' of method '{}'",
                        param.name, op_name
                    )));
                }
                if !route_path_names
                    .iter()
                    .all(|set| set.contains(&param.wire_name))
                {
                    return Err(IdlcError::rpc(format!(
                        "parameter '{}' is bound to path variable '{}' but it is not present in every route template of method '{}'",
                        param.name, param.wire_name, op_name
                    )));
                }
            } else {
                return Err(IdlcError::rpc(format!(
                    "parameter '{}' is annotated with @path but '{}' is not present in any route template of method '{}'",
                    param.name, param.wire_name, op_name
                )));
            }
        }
        HttpParamSource::Query => {}
        HttpParamSource::Body
            if matches!(
                direction,
                HttpParamDirection::In | HttpParamDirection::InOut
            ) => {}
        HttpParamSource::Header | HttpParamSource::Cookie | HttpParamSource::Body => {}
    }
    Ok(())
}

fn validate_route_bindings(
    op_name: &str,
    routes: &[HttpRoute],
    path_binding_count: &HashMap<String, usize>,
    query_binding_count: &HashMap<String, usize>,
) -> IdlcResult<()> {
    for route in routes {
        for route_param in &route.path_params {
            match path_binding_count.get(route_param).copied().unwrap_or(0) {
                0 => {
                    return Err(IdlcError::rpc(format!(
                        "route template variable '{}' has no matching request-side path parameter in method '{}'",
                        route_param, op_name
                    )));
                }
                1 => {}
                _ => {
                    return Err(IdlcError::rpc(format!(
                        "route template variable '{}' is bound by multiple request-side path parameters in method '{}'",
                        route_param, op_name
                    )));
                }
            }
        }
        for query_param in &route.query_params {
            match query_binding_count.get(query_param).copied().unwrap_or(0) {
                0 => {
                    return Err(IdlcError::rpc(format!(
                        "query template variable '{}' has no matching request-side query parameter in method '{}'",
                        query_param, op_name
                    )));
                }
                1 => {}
                _ => {
                    return Err(IdlcError::rpc(format!(
                        "query template variable '{}' is bound by multiple request-side query parameters in method '{}'",
                        query_param, op_name
                    )));
                }
            }
        }
    }
    Ok(())
}

fn validate_request_shape(
    op_name: &str,
    stream_kind: Option<HttpStreamKind>,
    request_path_params: &[HttpParam],
    request_query_params: &[HttpParam],
    request_header_params: &[HttpParam],
    request_cookie_params: &[HttpParam],
    request_body_params: &[HttpParam],
) -> IdlcResult<()> {
    if matches!(
        stream_kind,
        Some(HttpStreamKind::Client | HttpStreamKind::Bidi)
    ) && (!request_path_params.is_empty()
        || !request_query_params.is_empty()
        || !request_header_params.is_empty()
        || !request_cookie_params.is_empty())
    {
        let label = if matches!(stream_kind, Some(HttpStreamKind::Bidi)) {
            "@bidi_stream"
        } else {
            "@client_stream"
        };
        return Err(IdlcError::rpc(format!(
            "{label} method '{}' currently supports body parameters only",
            op_name
        )));
    }
    if request_body_params.len() != 1 && request_body_params.iter().any(|param| param.flatten) {
        return Err(IdlcError::rpc(format!(
            "@flatten requires exactly one request-side body parameter, but method '{}' has {}",
            op_name,
            request_body_params.len()
        )));
    }
    for param in request_body_params {
        if param.flatten && !matches!(param.source, HttpParamSource::Body) {
            return Err(IdlcError::rpc(format!(
                "@flatten can only be applied to body parameter '{}' of method '{}'",
                param.name, op_name
            )));
        }
    }
    Ok(())
}

fn validate_head_constraints(
    op_name: &str,
    method: HttpMethod,
    response_params: &[HttpParam],
    return_type: Option<&hir::TypeSpec>,
) -> IdlcResult<()> {
    if !matches!(method, HttpMethod::Head) {
        return Ok(());
    }
    if return_type.is_some() || !response_params.is_empty() {
        return Err(IdlcError::rpc(format!(
            "HEAD method '{}' must return void",
            op_name
        )));
    }
    Ok(())
}

fn http_method_name(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Patch => "PATCH",
        HttpMethod::Delete => "DELETE",
        HttpMethod::Head => "HEAD",
        HttpMethod::Options => "OPTIONS",
    }
}

fn readonly_attr_names(spec: &hir::ReadonlyAttrSpec) -> Vec<String> {
    match &spec.declarator {
        hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => vec![decl.0.clone()],
        hir::ReadonlyAttrDeclarator::RaisesExpr(_) => Vec::new(),
    }
}

fn attribute_path(name: &str) -> String {
    format!("/attribute/{name}")
}

fn default_path(module_path: &[String], interface_name: &str, method_name: &str) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    format!("/{}", parts.join("/"))
}

fn operation_id(module_path: &[String], interface_name: &str, method_name: &str) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    parts.join(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> hir::Specification {
        let typed = xidl_parser::parser::parser_text(source).expect("parse idl");
        hir::Specification::from_typed_ast_with_properties_and_path(
            typed,
            HashMap::new(),
            std::path::Path::new("input.idl"),
        )
        .expect("project hir")
    }

    #[test]
    fn projects_normalized_http_operations() {
        let spec = parse(
            r#"
            module api {
              interface CityApi {
                @get(path="/cities/{id}{?region}")
                string get_city(@path("id") in string id, @query("region") in string region, @header("X-Trace-Id") in string trace, out string etag);
              };
            };
            "#,
        );
        let doc = project(&spec).expect("http hir");
        let op = &doc.interfaces[0].operations[0];
        assert_eq!(op.method, HttpMethod::Get);
        assert_eq!(op.routes[0].path, "/cities/{id}");
        assert_eq!(op.routes[0].path_params, vec!["id".to_string()]);
        assert_eq!(op.routes[0].query_params, vec!["region".to_string()]);
        assert_eq!(op.request_path_params[0].wire_name, "id");
        assert_eq!(op.request_query_params[0].wire_name, "region");
        assert_eq!(op.request_header_params[0].wire_name, "X-Trace-Id");
        assert_eq!(op.response_body_params[0].name, "etag");
    }

    #[test]
    fn projects_attribute_operations_and_document_metadata() {
        let spec = parse(
            r#"
            #pragma xidlc package = "petstore"
            #pragma xidlc service "https://example.com" "prod"
            interface DeviceApi {
              @server_stream readonly attribute string status;
            };
            "#,
        );
        let doc = project(&spec).expect("http hir");
        assert_eq!(doc.document.package.as_deref(), Some("petstore"));
        assert_eq!(doc.document.servers[0].base_url, "https://example.com");
        let ops = &doc.interfaces[0].operations;
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].source, HttpOperationSource::AttributeGet);
        assert_eq!(ops[1].source, HttpOperationSource::AttributeWatch);
        assert_eq!(ops[1].stream.kind, Some(HttpStreamKind::Server));
    }

    #[test]
    fn projects_openapi_version_from_openapi_pragma_namespace() {
        let spec = parse(
            r#"
            #pragma xidlc openapi version = "2026.04"
            interface ReportApi {
              string ping();
            };
            "#,
        );
        let doc = project(&spec).expect("http hir");
        assert_eq!(doc.document.version.as_deref(), Some("2026.04"));
    }

    #[test]
    fn projects_security_and_media_type_inheritance() {
        let spec = parse(
            r#"
            @http_bearer
            @Consumes("application/msgpack")
            interface AuthApi {
              @Produces("application/x-www-form-urlencoded")
              string create(@body in string payload);
            };
            "#,
        );
        let doc = project(&spec).expect("http hir");
        let op = &doc.interfaces[0].operations[0];
        assert_eq!(op.request_content_type, "application/msgpack");
        assert_eq!(
            op.response_content_type,
            "application/x-www-form-urlencoded"
        );
        let security = op.security.as_ref().expect("security");
        assert_eq!(security.origin, semantics::HttpSecurityOrigin::Interface);
    }

    #[test]
    fn rejects_missing_query_template_binding_in_projection() {
        let spec = parse(
            r#"
            interface HttpApi {
              @get(path = "/users/{id}{?lang,region}")
              string get_user(@path("id") string id, @query("lang") string lang);
            };
            "#,
        );
        let err = project(&spec).expect_err("projection should fail");
        assert!(err.to_string().contains(
            "query template variable 'region' has no matching request-side query parameter"
        ));
    }

    #[test]
    fn rejects_invalid_stream_method_in_projection() {
        let spec = parse(
            r#"
            interface StreamApi {
              @server_stream
              @post(path = "/watch")
              string watch();
            };
            "#,
        );
        let err = project(&spec).expect_err("projection should fail");
        assert!(
            err.to_string()
                .contains("@server_stream method 'watch' must use GET")
        );
    }

    #[test]
    fn rejects_non_body_client_stream_inputs_in_projection() {
        let spec = parse(
            r#"
            interface StreamApi {
              @client_stream
              @post(path = "/upload/{id}")
              string upload(@path("id") string id, string payload);
            };
            "#,
        );
        let err = project(&spec).expect_err("projection should fail");
        assert!(
            err.to_string()
                .contains("@client_stream method 'upload' currently supports body parameters only")
        );
    }
}
