mod arrays;
mod constants;
mod deprecation;
mod inheritance;
mod recursive;
mod recursive_graph;
mod union_cases;

use crate::error::ParserResult;

pub(crate) use constants::IntegerDeclarations;
pub use constants::{INTEGER_IMPORTS_PROPERTY, IntegerExports, IntegerImports};
pub(crate) use union_cases::UnionCaseLabels;

pub(crate) fn analyze(
    spec: &mut crate::hir::Specification,
    imports: &IntegerImports,
) -> ParserResult<()> {
    arrays::ArrayDimensions::normalize(spec, imports)?;
    inheritance::analyze(spec)?;
    spec.inherit_module_deprecation();
    spec.mark_deprecated_references();
    recursive::annotate_recursive_members(spec);
    Ok(())
}

pub use recursive::recursive_schema_types;

#[cfg(test)]
mod tests;
