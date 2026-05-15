use xidl_parser::hir;

pub fn rust_passthrough_attrs_from_annotations(annotations: &[hir::Annotation]) -> Vec<String> {
    let mut out = Vec::new();
    for annotation in annotations {
        if let Some(attr_name) = rust_passthrough_attr_name(annotation) {
            let rendered = hir::annotation_params(annotation)
                .map(render_rust_passthrough_params)
                .unwrap_or_default();
            if rendered.is_empty() {
                out.push(attr_name);
            } else {
                out.push(format!("{attr_name}({rendered})"));
            }
        }
    }
    out
}

pub(crate) fn annotation_name_is_derive(annotation: &hir::Annotation) -> bool {
    hir::annotation_name(annotation)
        .map(|name| name.eq_ignore_ascii_case("derive"))
        .unwrap_or(false)
}

pub(crate) fn annotation_params(annotation: &hir::Annotation) -> Option<&hir::AnnotationParams> {
    hir::annotation_params(annotation)
}

fn rust_passthrough_attr_name(annotation: &hir::Annotation) -> Option<String> {
    let name = hir::annotation_name(annotation)?;
    let lower = name.to_ascii_lowercase();
    let attr = if lower.starts_with("rust-") {
        &name["rust-".len()..]
    } else if lower.starts_with("rust_") {
        &name["rust_".len()..]
    } else {
        return None;
    };
    if attr.is_empty() {
        None
    } else {
        Some(attr.to_string())
    }
}

fn render_rust_passthrough_params(params: &hir::AnnotationParams) -> String {
    match params {
        hir::AnnotationParams::Raw(value) => value.trim().to_string(),
        hir::AnnotationParams::ConstExpr(expr) => hir::normalize_annotation_params(
            &hir::AnnotationParams::ConstExpr(expr.clone()),
        )
        .get("value")
        .cloned()
        .unwrap_or_default(),
        hir::AnnotationParams::Positional(values) => values
            .iter()
            .map(|value| {
                hir::normalize_annotation_params(&hir::AnnotationParams::ConstExpr(value.clone()))
                    .get("value")
                    .cloned()
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(", "),
        hir::AnnotationParams::Params(values) => values
            .iter()
            .map(|value| {
                if let Some(expr) = &value.value {
                    let rendered = hir::normalize_annotation_params(
                        &hir::AnnotationParams::ConstExpr(expr.clone()),
                    )
                    .get("value")
                    .cloned()
                    .unwrap_or_default();
                    format!("{} = {}", value.ident, rendered)
                } else {
                    value.ident.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", "),
    }
}
