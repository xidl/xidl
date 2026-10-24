use crate::error::{IdlcError, IdlcResult};
use xidl_parser::hir;
use xidl_parser::rest_hir::semantics::{DeprecatedInfo, deprecated_info};

/// Documentation lines followed by the JSDoc `@deprecated` tag when the
/// declaration carries `@deprecated` (its own or inherited from its module).
pub(crate) fn ts_doc_lines(annotations: &[hir::Annotation]) -> IdlcResult<Vec<String>> {
    let mut lines = crate::generate::utils::doc_lines_from_annotations(annotations);
    lines.extend(ts_deprecation(annotations)?.map(|info| info.tag()));
    Ok(lines)
}

/// The `@deprecated` declared on a TypeScript-facing declaration.
pub(crate) fn ts_deprecation(
    annotations: &[hir::Annotation],
) -> IdlcResult<Option<DeprecatedInfo>> {
    deprecated_info(annotations).map_err(|reason| IdlcError::rpc(format!("@deprecated: {reason}")))
}

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
