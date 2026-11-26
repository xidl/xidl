use crate::error::{IdlcError, IdlcResult};
use xidl_parser::hir::{self, DeprecatedInfo, deprecated_info};

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

pub(crate) fn has_annotation(annotations: &[hir::Annotation], target: &str) -> bool {
    hir::has_annotation(annotations, target)
}

pub(crate) fn has_optional_annotation(annotations: &[hir::Annotation]) -> bool {
    hir::has_optional_annotation(annotations)
}
