//! Shared HIR annotation query helpers.

use crate::hir::{Annotation, annotation_name};

/// Returns `true` when `annotations` contains an annotation named `target` (ASCII case-insensitive).
pub fn has_annotation(annotations: &[Annotation], target: &str) -> bool {
    annotations.iter().any(|annotation| {
        annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case(target))
            .unwrap_or(false)
    })
}

/// Finds the first annotation named `target` (ASCII case-insensitive).
pub fn find_annotation<'a>(annotations: &'a [Annotation], target: &str) -> Option<&'a Annotation> {
    annotations.iter().find(|annotation| {
        annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case(target))
            .unwrap_or(false)
    })
}

/// Returns `true` when `annotations` contains `@optional`.
pub fn has_optional_annotation(annotations: &[Annotation]) -> bool {
    annotations.iter().any(|annotation| {
        matches!(annotation, Annotation::Optional { .. })
            || annotation_name(annotation)
                .map(|name| name.eq_ignore_ascii_case("optional"))
                .unwrap_or(false)
    })
}
