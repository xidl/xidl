mod inheritance;
mod recursive;
mod recursive_graph;
mod switch_enums;

use crate::error::ParserResult;

pub(crate) fn analyze(spec: &mut crate::hir::Specification) -> ParserResult<()> {
    inheritance::analyze(spec)?;
    recursive::annotate_recursive_members(spec);
    switch_enums::annotate_switch_enums(spec);
    Ok(())
}

pub use recursive::recursive_schema_types;

#[cfg(test)]
mod tests;
