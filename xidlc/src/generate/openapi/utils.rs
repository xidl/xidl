use crate::generate::http_hir::semantics::{
    annotation_name, annotation_params, normalize_annotation_params,
};
use crate::generate::utils::doc_lines_from_annotations;
use crate::openapi::path::HttpMethod as OpenApiHttpMethod;
use xidl_parser::hir;

/// Returns the lines of documentation from the given annotations.
pub fn doc_text(annotations: &[hir::Annotation]) -> Option<String> {
    let lines = doc_lines_from_annotations(annotations);
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

/// Returns the name of the declarator.
pub fn declarator_name(decl: &hir::Declarator) -> String {
    match decl {
        hir::Declarator::SimpleDeclarator(simple) => simple.0.clone(),
        hir::Declarator::ArrayDeclarator(array) => array.ident.clone(),
    }
}

/// Returns a scoped name by joining the module path and the identifier with dots.
pub fn scoped_name(module_path: &[String], ident: &str) -> String {
    if module_path.is_empty() {
        ident.to_string()
    } else {
        let mut parts = module_path.to_vec();
        parts.push(ident.to_string());
        parts.join(".")
    }
}

/// Returns an operation ID by joining the module path, interface name, and method name with dots.
pub fn operation_id(module_path: &[String], interface_name: &str, method_name: &str) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    parts.join(".")
}

/// Returns the value of the `@name` annotation if present.
pub fn field_rename(annotations: &[hir::Annotation]) -> Option<String> {
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

/// Converts an OpenAPI HTTP method to its string representation.
pub fn openapi_method_name(method: &OpenApiHttpMethod) -> &'static str {
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

/// Converts a path template to OpenAPI format.
pub fn openapi_path_template(path: &str) -> String {
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
