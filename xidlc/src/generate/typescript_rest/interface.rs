use super::model::{
    ClientClassContext, MethodModel, PathParamContext, RequestPayloadEntry, SecurityContext,
    TsHttpBlocks, ValueParamContext,
};
use super::server::render_server_block;
use crate::error::{IdlcError, IdlcResult};
use crate::generate::typescript::TypescriptRenderer;
use crate::generate::typescript::definition::TypeRefTarget;
use crate::generate::typescript::definition::contexts::{
    ClientParamContext, ParamDeclContext, RequestContext, RequestZodContext, TsType,
};
use crate::generate::typescript::definition::names::{method_struct_prefix, scoped_name, ts_ident};
use crate::generate::typescript::definition::type_expr::{
    ts_type_for_type_spec, zod_schema_for_type_spec,
};
use xidl_parser::hir;
use xidl_parser::rest_hir::{
    HttpInputBinding, HttpOperation, HttpOutputBinding, HttpOutputSource, HttpRequestBodyShape,
    HttpResponseBodyShape, RestHirDocument,
    semantics::{HttpSecurityRequirement, HttpStreamCodec, HttpStreamKind},
};

pub(crate) fn render_interface(
    interface: &hir::InterfaceDcl,
    module_path: &[String],
    renderer: &TypescriptRenderer,
    rest_hir: &RestHirDocument,
) -> IdlcResult<TsHttpBlocks> {
    let hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
        return Ok(TsHttpBlocks::default());
    };
    let Some(http_interface) = rest_hir.find_interface(module_path, &def.header.ident) else {
        return Ok(TsHttpBlocks::default());
    };
    let methods = http_interface
        .operations
        .iter()
        .map(|op| build_method_model(def.header.ident.as_str(), module_path, op))
        .collect::<IdlcResult<Vec<_>>>()?;
    let mut out = TsHttpBlocks::default();
    for method in &methods {
        render_request_types(&mut out, method, module_path, renderer)?;
        render_response_types(&mut out, method, module_path, renderer)?;
    }
    out.client.push(
        renderer.render_template(
            "http/client_class.ts.j2",
            &ClientClassContext {
                client_name: ts_ident(&def.header.ident), // Template adds 'Client'
                methods: methods
                    .iter()
                    .map(|method| MethodModel {
                        name: method.name.clone(),
                        params: method.params.clone(),
                        request_name: method.request_name.clone(),
                        request_schema_ref: method.request_schema_ref.clone(),
                        body_schema_ref: method.body_schema_ref.clone(),
                        request_payload: method.request_payload.clone(),
                        response_name: method.response_name.clone(),
                        response_schema_ref: method.response_schema_ref.clone(),
                        request_content_type: method.request_content_type.clone(),
                        response_content_type: method.response_content_type.clone(),
                        path: method.path.clone(),
                        http_method: method.http_method.clone(),
                        path_params: method.path_params.clone(),
                        query_params: method.query_params.clone(),
                        header_params: method.header_params.clone(),
                        cookie_params: method.cookie_params.clone(),
                        response_header_params: method.response_header_params.clone(),
                        response_cookie_params: method.response_cookie_params.clone(),
                        body_entries: method.body_entries.clone(),
                        body_single: method.body_single.clone(),
                        return_ty: method.return_ty.clone(),
                        response_body_mode: method.response_body_mode.clone(),
                        response_body_entries: method.response_body_entries.clone(),
                        stream_item_ty: method.stream_item_ty.clone(),
                        stream_item_schema_ref: method.stream_item_schema_ref.clone(),
                        client_stream_item_ty: method.client_stream_item_ty.clone(),
                        is_server_stream: method.is_server_stream,
                        is_client_stream: method.is_client_stream,
                        security: method.security.clone(),
                        request_fields: method.request_fields.clone(),
                        response_fields: method.response_fields.clone(),
                    })
                    .map(MethodModel::into_client_context)
                    .collect(),
            },
        )?,
    );
    out.server.push(render_server_block(
        &def.header.ident,
        module_path,
        methods,
        renderer,
    )?);
    Ok(out)
}

fn build_method_model(
    interface_name: &str,
    module_path: &[String],
    op: &HttpOperation,
) -> IdlcResult<MethodModel> {
    validate_stream_support(op)?;
    let prefix = method_struct_prefix(interface_name, &op.meta.name);

    let request_fields = op
        .signature
        .params
        .iter()
        .filter(|p| {
            matches!(
                p.direction,
                xidl_parser::rest_hir::HttpSignatureParamDirection::In
                    | xidl_parser::rest_hir::HttpSignatureParamDirection::InOut
            )
        })
        .map(|p| ParamDeclContext {
            prop: crate::generate::typescript::definition::names::ts_prop_name(&p.name),
            ty: ts_type_for_type_spec(&p.ty, module_path, TypeRefTarget::Types),
            schema: zod_schema_for_type_spec(&p.ty, module_path),
            optional: p.annotations.iter().any(|a| {
                matches!(
                    a,
                    xidl_parser::rest_hir::HttpSignatureParamAnnotation::Optional
                )
            }),
            doc: Vec::new(),
        })
        .collect::<Vec<_>>();

    let response_fields = build_response_fields(op, module_path);
    let request_name = (!request_fields.is_empty()).then(|| format!("{prefix}Request"));
    let response_name = (!response_fields.is_empty() && !only_direct_return(op))
        .then(|| format!("{prefix}Response"));
    let request_schema_ref = request_name
        .as_ref()
        .filter(|_| !matches!(op.meta.stream.kind, Some(HttpStreamKind::Client)))
        .map(|name| scoped_name(module_path, name));
    let response_schema_ref = response_name
        .as_ref()
        .map(|name| scoped_name(module_path, name));

    let return_ty = response_name
        .as_ref()
        .map(|name| TsType::ScopedName(scoped_name(module_path, name)))
        .or_else(|| {
            op.signature
                .return_type
                .as_ref()
                .map(|ty| ts_type_for_type_spec(ty, module_path, TypeRefTarget::Client))
        })
        .unwrap_or(TsType::Void);

    let client_stream_item_ty = build_client_stream_ty(op, module_path, &request_name);
    let path = op
        .meta
        .routes
        .first()
        .map(|route| route.path.clone())
        .unwrap_or_else(|| "/".to_string());

    let request_payload = op
        .signature
        .params
        .iter()
        .filter(|p| {
            matches!(
                p.direction,
                xidl_parser::rest_hir::HttpSignatureParamDirection::In
                    | xidl_parser::rest_hir::HttpSignatureParamDirection::InOut
            )
        })
        .map(|p| RequestPayloadEntry {
            raw_name: p.name.clone(),
            access: ts_ident(&p.name),
        })
        .collect();

    let (body_entries, body_single) = match &op.http.request.body.shape {
        HttpRequestBodyShape::Empty => (Vec::new(), None),
        HttpRequestBodyShape::SingleValue {
            source_param,
            flatten,
            ..
        } => {
            let entries = vec![RequestPayloadEntry {
                raw_name: source_param.clone(),
                access: ts_ident(source_param),
            }];
            let single = flatten.then(|| ts_ident(source_param));
            (entries, single)
        }
        HttpRequestBodyShape::Object { fields } => {
            let entries = fields
                .iter()
                .map(|f| RequestPayloadEntry {
                    raw_name: f.source_param.clone(),
                    access: ts_ident(&f.source_param),
                })
                .collect();
            (entries, None)
        }
        HttpRequestBodyShape::Stream { source_param, .. } => {
            let entries = vec![RequestPayloadEntry {
                raw_name: source_param.clone(),
                access: ts_ident(source_param),
            }];
            (entries, None)
        }
    };

    let response_body_entries = match &op.http.response.body.shape {
        HttpResponseBodyShape::Empty => Vec::new(),
        HttpResponseBodyShape::ReturnOnly { .. } => vec![RequestPayloadEntry {
            raw_name: "return".to_string(),
            access: "return".to_string(),
        }],
        HttpResponseBodyShape::SingleValue { source, .. } => {
            let name = match source {
                HttpOutputSource::ReturnValue => "return".to_string(),
                HttpOutputSource::Param { name } => name.clone(),
            };
            vec![RequestPayloadEntry {
                raw_name: name.clone(),
                access: ts_ident(&name),
            }]
        }
        HttpResponseBodyShape::Object { fields } => fields
            .iter()
            .map(|f| {
                let name = match &f.source {
                    HttpOutputSource::ReturnValue => "return".to_string(),
                    HttpOutputSource::Param { name } => name.clone(),
                };
                RequestPayloadEntry {
                    raw_name: f.field_name.clone(),
                    access: ts_ident(&name),
                }
            })
            .collect(),
        HttpResponseBodyShape::Stream {
            item_source: source,
            ..
        } => {
            let name = match source {
                HttpOutputSource::ReturnValue => "return".to_string(),
                HttpOutputSource::Param { name } => name.clone(),
            };
            vec![RequestPayloadEntry {
                raw_name: name.clone(),
                access: ts_ident(&name),
            }]
        }
    };

    let body_schema_ref = match &op.http.request.body.shape {
        HttpRequestBodyShape::Empty => None,
        HttpRequestBodyShape::SingleValue { .. } => {
            // TODO: handle inline schemas
            Some(scoped_name(module_path, "FIXME"))
        }
        HttpRequestBodyShape::Object { .. } => request_schema_ref.clone(),
        HttpRequestBodyShape::Stream { .. } => {
            // TODO: handle inline schemas
            Some(scoped_name(module_path, "FIXME"))
        }
    };

    let stream_item_schema_ref =
        if let HttpResponseBodyShape::Stream { .. } = &op.http.response.body.shape {
            // TODO: handle inline schemas
            Some(scoped_name(module_path, "FIXME"))
        } else {
            None
        };

    Ok(MethodModel {
        name: ts_ident(&op.meta.name),
        params: build_client_params(op, module_path, &request_name),
        request_name,
        request_schema_ref,
        body_schema_ref,
        request_payload,
        response_name,
        response_schema_ref,
        request_content_type: op
            .http
            .request
            .body
            .content_type
            .clone()
            .unwrap_or_else(|| "application/json".to_string()),
        response_content_type: op
            .http
            .response
            .body
            .content_type
            .clone()
            .unwrap_or_else(|| "application/json".to_string()),
        path: path.clone(),
        http_method: http_method_name(op.meta.method).to_string(),
        path_params: build_value_params(&op.http.request.path)
            .into_iter()
            .map(|param| PathParamContext {
                catch_all: path.contains(&format!("{{*{}}}", param.raw_name)),
                template_name: param.raw_name,
                access: param.access,
                key_name: param.key_name,
            })
            .collect(),
        query_params: build_value_params(&op.http.request.query),
        header_params: build_value_params(&op.http.request.header),
        cookie_params: build_value_params(&op.http.request.cookie),
        response_header_params: build_response_value_params(&op.http.response.header),
        response_cookie_params: build_response_value_params(&op.http.response.cookie),
        body_entries,
        body_single,
        return_ty,
        response_body_mode: build_response_body_mode(op).to_string(),
        response_body_entries,
        stream_item_ty: build_server_stream_ty(op, module_path),
        stream_item_schema_ref,
        client_stream_item_ty,
        is_server_stream: matches!(op.meta.stream.kind, Some(HttpStreamKind::Server)),
        is_client_stream: matches!(op.meta.stream.kind, Some(HttpStreamKind::Client)),
        security: security_contexts(op),
        request_fields,
        response_fields,
    })
}

fn render_request_types(
    out: &mut TsHttpBlocks,
    method: &MethodModel,
    module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<()> {
    let Some(name) = method.request_name.as_ref() else {
        return Ok(());
    };
    out.types.push(renderer.render_template(
        "http/request.d.ts.j2",
        &RequestContext {
            name: name.to_string(),
            params: method.request_fields.clone(),
            doc: Vec::new(),
        },
    )?);
    out.zod.push(renderer.render_template(
        "http/request.zod.ts.j2",
        &RequestZodContext {
            schema_name: name.clone(), // Template adds 'Schema'
            name: name.to_string(),
            params: method.request_fields.clone(),
        },
    )?);
    let _ = module_path;
    Ok(())
}

fn render_response_types(
    out: &mut TsHttpBlocks,
    method: &MethodModel,
    module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<()> {
    let Some(name) = method.response_name.as_ref() else {
        return Ok(());
    };
    let _ = module_path;
    out.types.push(renderer.render_template(
        "http/request.d.ts.j2",
        &RequestContext {
            name: name.to_string(),
            params: method.response_fields.clone(),
            doc: Vec::new(),
        },
    )?);
    out.zod.push(renderer.render_template(
        "http/request.zod.ts.j2",
        &RequestZodContext {
            schema_name: name.clone(), // Template adds 'Schema'
            name: name.to_string(),
            params: method.response_fields.clone(),
        },
    )?);
    Ok(())
}

fn build_response_fields(op: &HttpOperation, module_path: &[String]) -> Vec<ParamDeclContext> {
    let mut fields = Vec::new();
    if let Some(ty) = &op.signature.return_type {
        fields.push(ParamDeclContext {
            prop: "return".to_string(),
            ty: ts_type_for_type_spec(ty, module_path, TypeRefTarget::Types),
            schema: zod_schema_for_type_spec(ty, module_path),
            optional: false,
            doc: Vec::new(),
        });
    }
    for param in &op.signature.params {
        if matches!(
            param.direction,
            xidl_parser::rest_hir::HttpSignatureParamDirection::Out
                | xidl_parser::rest_hir::HttpSignatureParamDirection::InOut
        ) {
            fields.push(ParamDeclContext {
                prop: crate::generate::typescript::definition::names::ts_prop_name(&param.name),
                ty: ts_type_for_type_spec(&param.ty, module_path, TypeRefTarget::Types),
                schema: zod_schema_for_type_spec(&param.ty, module_path),
                optional: param.annotations.iter().any(|a| {
                    matches!(
                        a,
                        xidl_parser::rest_hir::HttpSignatureParamAnnotation::Optional
                    )
                }),
                doc: Vec::new(),
            });
        }
    }
    fields
}

fn build_client_params(
    op: &HttpOperation,
    module_path: &[String],
    request_name: &Option<String>,
) -> Vec<ClientParamContext> {
    if matches!(op.meta.stream.kind, Some(HttpStreamKind::Client)) {
        return vec![ClientParamContext {
            name: "stream".to_string(),
            ty: build_client_stream_ty(op, module_path, request_name).unwrap_or(TsType::Void),
        }];
    }
    op.signature
        .params
        .iter()
        .filter(|p| {
            matches!(
                p.direction,
                xidl_parser::rest_hir::HttpSignatureParamDirection::In
                    | xidl_parser::rest_hir::HttpSignatureParamDirection::InOut
            )
        })
        .map(|p| {
            let base = ts_type_for_type_spec(&p.ty, module_path, TypeRefTarget::Client);
            let optional = p.annotations.iter().any(|a| {
                matches!(
                    a,
                    xidl_parser::rest_hir::HttpSignatureParamAnnotation::Optional
                )
            });
            let ty = if optional {
                TsType::Optional(Box::new(base))
            } else {
                base
            };
            ClientParamContext {
                name: ts_ident(&p.name),
                ty,
            }
        })
        .collect()
}

fn build_value_params(bindings: &[HttpInputBinding]) -> Vec<ValueParamContext> {
    bindings
        .iter()
        .map(|b| ValueParamContext {
            raw_name: b.wire_name.clone(),
            access: ts_ident(&b.source_param),
            key_name: b.source_param.clone(),
            optional: b.optional,
            is_multi: matches!(b.ty, hir::TypeSpec::SequenceType(_)),
        })
        .collect()
}

fn build_response_value_params(bindings: &[HttpOutputBinding]) -> Vec<ValueParamContext> {
    bindings
        .iter()
        .map(|b| {
            let name = match &b.source {
                HttpOutputSource::ReturnValue => "return".to_string(),
                HttpOutputSource::Param { name } => name.clone(),
            };
            ValueParamContext {
                raw_name: b.wire_name.clone(),
                access: crate::generate::typescript::definition::names::ts_prop_name(&name),
                key_name: name,
                optional: false,
                is_multi: matches!(b.ty, hir::TypeSpec::SequenceType(_)),
            }
        })
        .collect()
}

fn build_server_stream_ty(op: &HttpOperation, module_path: &[String]) -> Option<TsType> {
    if let HttpResponseBodyShape::Stream { item_ty, .. } = &op.http.response.body.shape {
        Some(ts_type_for_type_spec(
            item_ty,
            module_path,
            TypeRefTarget::Client,
        ))
    } else {
        None
    }
}

fn build_client_stream_ty(
    op: &HttpOperation,
    module_path: &[String],
    request_name: &Option<String>,
) -> Option<TsType> {
    if let HttpRequestBodyShape::Stream {
        item_ty,
        source_param,
        ..
    } = &op.http.request.body.shape
    {
        // Try to find if this source_param was a flattened single param
        let was_flattened = op.signature.params.iter().any(|p| {
            p.name == *source_param
                && p.annotations.iter().any(|a| {
                    matches!(
                        a,
                        xidl_parser::rest_hir::HttpSignatureParamAnnotation::Flatten
                    )
                })
        });

        let item_ty_str = if was_flattened {
            ts_type_for_type_spec(item_ty, module_path, TypeRefTarget::Client)
        } else if let Some(name) = request_name {
            TsType::ScopedName(scoped_name(module_path, name))
        } else {
            TsType::Void
        };
        Some(TsType::AsyncIterable(Box::new(item_ty_str)))
    } else {
        None
    }
}

fn build_response_body_mode(op: &HttpOperation) -> &'static str {
    match &op.http.response.body.shape {
        HttpResponseBodyShape::Empty => "none",
        HttpResponseBodyShape::ReturnOnly { .. } => "return",
        HttpResponseBodyShape::SingleValue { source, .. } => {
            if matches!(source, HttpOutputSource::ReturnValue) {
                "return"
            } else {
                "object"
            }
        }
        HttpResponseBodyShape::Object { .. } => "object",
        HttpResponseBodyShape::Stream { .. } => "return", // Stream is handled specially but 'return' mode is closest for result handling
    }
}

fn validate_stream_support(op: &HttpOperation) -> IdlcResult<()> {
    match op.meta.stream.kind {
        Some(HttpStreamKind::Server) if op.meta.stream.codec != HttpStreamCodec::Sse => {
            Err(IdlcError::rpc(format!(
                "typescript-rest currently supports only SSE for @server_stream methods: '{}'",
                op.meta.name
            )))
        }
        Some(HttpStreamKind::Client) if op.meta.stream.codec != HttpStreamCodec::Ndjson => {
            Err(IdlcError::rpc(format!(
                "typescript-rest currently supports only NDJSON for @client_stream methods: '{}'",
                op.meta.name
            )))
        }
        Some(HttpStreamKind::Bidi) => Err(IdlcError::rpc(format!(
            "typescript-rest currently does not support @bidi_stream methods: '{}'",
            op.meta.name
        ))),
        _ => Ok(()),
    }
}

fn only_direct_return(op: &HttpOperation) -> bool {
    op.signature.return_type.is_some()
        && op.http.response.header.is_empty()
        && op.http.response.cookie.is_empty()
        && matches!(
            op.http.response.body.shape,
            HttpResponseBodyShape::ReturnOnly { .. }
                | HttpResponseBodyShape::Stream {
                    item_source: HttpOutputSource::ReturnValue,
                    ..
                }
        )
}

fn http_method_name(method: xidl_parser::rest_hir::HttpMethod) -> &'static str {
    match method {
        xidl_parser::rest_hir::HttpMethod::Get => "GET",
        xidl_parser::rest_hir::HttpMethod::Post => "POST",
        xidl_parser::rest_hir::HttpMethod::Put => "PUT",
        xidl_parser::rest_hir::HttpMethod::Patch => "PATCH",
        xidl_parser::rest_hir::HttpMethod::Delete => "DELETE",
        xidl_parser::rest_hir::HttpMethod::Head => "HEAD",
        xidl_parser::rest_hir::HttpMethod::Options => "OPTIONS",
    }
}

fn security_contexts(op: &HttpOperation) -> Vec<SecurityContext> {
    op.meta
        .security
        .as_ref()
        .map(|profile| {
            profile
                .requirements
                .iter()
                .map(|value| match value {
                    HttpSecurityRequirement::HttpBasic => SecurityContext {
                        kind: "http_basic".to_string(),
                        location: None,
                        name: None,
                        realm: op.meta.basic_auth_realm.clone(),
                        scopes: Vec::new(),
                    },
                    HttpSecurityRequirement::HttpBearer => SecurityContext {
                        kind: "http_bearer".to_string(),
                        location: None,
                        name: None,
                        realm: None,
                        scopes: Vec::new(),
                    },
                    HttpSecurityRequirement::ApiKey { location, name } => SecurityContext {
                        kind: "api_key".to_string(),
                        location: Some(format!("{location:?}").to_ascii_lowercase()),
                        name: Some(name.clone()),
                        realm: None,
                        scopes: Vec::new(),
                    },
                    HttpSecurityRequirement::OAuth2 { scopes } => SecurityContext {
                        kind: "oauth2".to_string(),
                        location: None,
                        name: None,
                        realm: None,
                        scopes: scopes.clone(),
                    },
                })
                .collect()
        })
        .unwrap_or_default()
}
