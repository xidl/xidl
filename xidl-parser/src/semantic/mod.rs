mod arrays;
mod inheritance;
mod recursive;
mod recursive_graph;

use crate::error::ParserResult;

pub(crate) fn analyze(spec: &mut crate::hir::Specification) -> ParserResult<()> {
    arrays::ArrayDimensions::normalize(spec)?;
    inheritance::analyze(spec)?;
    recursive::annotate_recursive_members(spec);
    Ok(())
}

pub use recursive::recursive_schema_types;

#[cfg(test)]
mod tests;
