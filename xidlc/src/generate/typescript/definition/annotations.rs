use xidl_parser::hir;

pub(crate) fn annotation_name(annotation: &hir::Annotation) -> Option<&str> {
    hir::annotation_name(annotation)
}

pub(crate) fn annotation_params(annotation: &hir::Annotation) -> Option<&hir::AnnotationParams> {
    hir::annotation_params(annotation)
}

pub(crate) fn has_annotation(annotations: &[hir::Annotation], target: &str) -> bool {
    annotations.iter().any(|annotation| {
        annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case(target))
            .unwrap_or(false)
    })
}

pub(crate) fn has_optional_annotation(annotations: &[hir::Annotation]) -> bool {
    annotations.iter().any(|annotation| {
        matches!(annotation, hir::Annotation::Optional { .. })
            || annotation_name(annotation)
                .map(|name| name.eq_ignore_ascii_case("optional"))
                .unwrap_or(false)
    })
}
