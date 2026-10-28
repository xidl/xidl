mod bounds;
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

pub(crate) use bounds::CollectionBounds;

pub(crate) struct Analysis;

impl Analysis {
    pub(crate) fn run(
        spec: &mut crate::hir::Specification,
        imports: &IntegerImports,
    ) -> ParserResult<()> {
        CollectionBounds::normalize(spec, imports)?;
        Self::structure(spec)
    }

    pub(crate) fn structure(spec: &mut crate::hir::Specification) -> ParserResult<()> {
        inheritance::analyze(spec)?;
        spec.inherit_module_deprecation();
        spec.mark_deprecated_references();
        recursive::annotate_recursive_members(spec);
        Ok(())
    }
}

pub use recursive::recursive_schema_types;

#[cfg(test)]
mod tests;
