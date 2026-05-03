use xidl_parser::http_hir::HttpMethod;
use xidl_parser::http_hir::semantics::{
    annotation_name, annotation_params, normalize_annotation_params,
};
use crate::openapi::path::HttpMethod as OpenApiHttpMethod;
use xidl_parser::hir;

pub(crate) fn declarator_name(decl: &hir::Declarator) -> String {
    match decl {
        hir::Declarator::SimpleDeclarator(simple) => simple.0.clone(),
        hir::Declarator::ArrayDeclarator(array) => array.ident.clone(),
    }
}

pub(crate) fn scoped_name(module_path: &[String], ident: &str) -> String {
    if module_path.is_empty() {
        ident.to_string()
    } else {
        let mut parts = module_path.to_vec();
        parts.push(ident.to_string());
        parts.join(".")
    }
}

pub(crate) fn operation_id(
    module_path: &[String],
    interface_name: &str,
    method_name: &str,
) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    parts.join(".")
}

pub(crate) fn field_rename(annotations: &[hir::Annotation]) -> Option<String> {
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if !name.eq_ignore_ascii_case("name") {
            continue;
        }
        let value = annotation_params(annotation)
            .map(normalize_annotation_params)
            .and_then(|params| {
                params
                    .get("value")
                    .cloned()
                    .or_else(|| params.get("name").cloned())
            });
        if value.is_some() {
            return value;
        }
    }
    None
}

pub(crate) fn openapi_path_template(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut in_param = false;
    let mut buf = String::new();
    for ch in path.chars() {
        match ch {
            '{' if !in_param => {
                in_param = true;
                buf.clear();
                out.push('{');
            }
            '}' if in_param => {
                out.push_str(buf.strip_prefix('*').unwrap_or(&buf));
                out.push('}');
                in_param = false;
            }
            _ if in_param => buf.push(ch),
            _ => out.push(ch),
        }
    }
    out
}

pub(crate) fn method_to_openapi(method: HttpMethod) -> OpenApiHttpMethod {
    match method {
        HttpMethod::Get => OpenApiHttpMethod::Get,
        HttpMethod::Post => OpenApiHttpMethod::Post,
        HttpMethod::Put => OpenApiHttpMethod::Put,
        HttpMethod::Patch => OpenApiHttpMethod::Patch,
        HttpMethod::Delete => OpenApiHttpMethod::Delete,
        HttpMethod::Head => OpenApiHttpMethod::Head,
        HttpMethod::Options => OpenApiHttpMethod::Options,
    }
}

pub(crate) fn openapi_method_name(method: &OpenApiHttpMethod) -> &'static str {
    match method {
        OpenApiHttpMethod::Get => "get",
        OpenApiHttpMethod::Post => "post",
        OpenApiHttpMethod::Put => "put",
        OpenApiHttpMethod::Patch => "patch",
        OpenApiHttpMethod::Delete => "delete",
        OpenApiHttpMethod::Head => "head",
        OpenApiHttpMethod::Options => "options",
        _ => "unknown",
    }
}
