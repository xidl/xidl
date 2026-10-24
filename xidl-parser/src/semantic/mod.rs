mod deprecation;
mod inheritance;
mod recursive;
mod recursive_graph;

use crate::error::ParserResult;

pub(crate) fn analyze(spec: &mut crate::hir::Specification) -> ParserResult<()> {
    inheritance::analyze(spec)?;
    spec.inherit_module_deprecation();
    spec.mark_deprecated_references();
    recursive::annotate_recursive_members(spec);
    Ok(())
}

pub use recursive::recursive_schema_types;

#[cfg(test)]
mod tests;
