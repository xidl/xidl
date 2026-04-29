use crate::generate::typescript::definition::annotations::has_annotation;
use crate::generate::typescript::definition::method::{MethodInfo, ParamInfo, ReturnType};
use xidl_parser::hir;

use super::http::method_http_code;
use super::method::HttpMethod;
use super::names::{default_path, method_struct_prefix, scoped_name, ts_ident};

pub(crate) fn render_attr(
    attr: &hir::AttrDcl,
    interface_name: &str,
    module_path: &[String],
) -> Vec<MethodInfo> {
    let emit_watch = has_annotation(&attr.annotations, "server_stream");
    let doc = crate::generate::utils::doc_lines_from_annotations(&attr.annotations);
    match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => readonly_attr_names(spec)
            .into_iter()
            .flat_map(|names| {
                readonly_methods(
                    spec,
                    emit_watch,
                    names.raw,
                    interface_name,
                    module_path,
                    &doc,
                )
            })
            .collect(),
        hir::AttrDclInner::AttrSpec(spec) => match &spec.declarator {
            hir::AttrDeclarator::SimpleDeclarator(list) => list
                .iter()
                .flat_map(|decl| {
                    read_write_methods(
                        spec,
                        emit_watch,
                        decl.0.clone(),
                        interface_name,
                        module_path,
                        &doc,
                    )
                })
                .collect(),
            hir::AttrDeclarator::WithRaises { declarator, .. } => read_write_methods(
                spec,
                emit_watch,
                declarator.0.clone(),
                interface_name,
                module_path,
                &doc,
            ),
        },
    }
}

fn readonly_methods(
    spec: &hir::ReadonlyAttrSpec,
    emit_watch: bool,
    raw: String,
    interface_name: &str,
    module_path: &[String],
    doc: &[String],
) -> Vec<MethodInfo> {
    let mut methods = vec![getter(
        raw.clone(),
        spec.ty.clone(),
        interface_name,
        module_path,
        doc,
        false,
    )];
    if emit_watch {
        methods.push(getter(
            format!("watch_attribute_{raw}"),
            spec.ty.clone(),
            interface_name,
            module_path,
            doc,
            true,
        ));
    }
    methods
}

fn read_write_methods(
    spec: &hir::AttrSpec,
    emit_watch: bool,
    raw_name: String,
    interface_name: &str,
    module_path: &[String],
    doc: &[String],
) -> Vec<MethodInfo> {
    let mut out = vec![getter(
        raw_name.clone(),
        spec.ty.clone(),
        interface_name,
        module_path,
        doc,
        false,
    )];
    out.push(setter(
        &raw_name,
        spec.ty.clone(),
        interface_name,
        module_path,
        doc,
    ));
    if emit_watch {
        out.push(getter(
            format!("watch_attribute_{raw_name}"),
            spec.ty.clone(),
            interface_name,
            module_path,
            doc,
            true,
        ));
    }
    out
}

fn getter(
    raw_name: String,
    ty: hir::TypeSpec,
    interface_name: &str,
    module_path: &[String],
    doc: &[String],
    is_server_stream: bool,
) -> MethodInfo {
    MethodInfo {
        name: ts_ident(&raw_name),
        params: Vec::new(),
        ret: ReturnType::new(ty),
        response_name: None,
        http_method: method_http_code(HttpMethod::Get),
        path: default_path(module_path, interface_name, &raw_name),
        request_name: None,
        request_schema_ref: None,
        path_params: Vec::new(),
        query_params: Vec::new(),
        header_params: Vec::new(),
        cookie_params: Vec::new(),
        body_params: Vec::new(),
        output_params: Vec::new(),
        is_server_stream,
        is_client_stream: false,
        doc: doc.to_vec(),
    }
}

fn setter(
    raw_name: &str,
    ty: hir::TypeSpec,
    interface_name: &str,
    module_path: &[String],
    doc: &[String],
) -> MethodInfo {
    let setter_raw = format!("set_{raw_name}");
    let param = ParamInfo {
        name: "value".to_string(),
        raw_name: "value".to_string(),
        wire_name: "value".to_string(),
        ty,
        optional: false,
        doc: doc.to_vec(),
    };
    let request_name = Some(format!(
        "{}Request",
        method_struct_prefix(interface_name, &setter_raw)
    ));
    let request_schema_ref = request_name.as_ref().map(|name| {
        let full = scoped_name(module_path, name);
        format!("zodSchemas.{full}Schema")
    });
    MethodInfo {
        name: ts_ident(&setter_raw),
        params: vec![param.clone()],
        ret: ReturnType::void(),
        response_name: None,
        http_method: method_http_code(HttpMethod::Post),
        path: default_path(module_path, interface_name, &setter_raw),
        request_name,
        request_schema_ref,
        path_params: Vec::new(),
        query_params: Vec::new(),
        header_params: Vec::new(),
        cookie_params: Vec::new(),
        body_params: vec![param],
        output_params: Vec::new(),
        is_server_stream: false,
        is_client_stream: false,
        doc: doc.to_vec(),
    }
}

struct AttrNames {
    raw: String,
}

fn readonly_attr_names(spec: &hir::ReadonlyAttrSpec) -> Vec<AttrNames> {
    match &spec.declarator {
        hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => vec![AttrNames {
            raw: decl.0.clone(),
        }],
        hir::ReadonlyAttrDeclarator::RaisesExpr(_) => Vec::new(),
    }
}
