mod arrays;
mod deprecation;
mod inheritance;
mod recursive;
mod recursive_graph;
mod union_cases;

use crate::error::ParserResult;

pub(crate) use union_cases::UnionCaseLabels;

pub(crate) fn analyze(spec: &mut crate::hir::Specification) -> ParserResult<()> {
    arrays::ArrayDimensions::normalize(spec)?;
    inheritance::analyze(spec)?;
    spec.inherit_module_deprecation();
    spec.mark_deprecated_references();
    recursive::annotate_recursive_members(spec);
    Ok(())
}

pub use recursive::recursive_schema_types;

#[cfg(test)]
mod tests;
