mod recursive;
mod recursive_graph;

pub(crate) fn analyze(spec: &mut crate::hir::Specification) {
    recursive::annotate_recursive_members(spec);
}

pub use recursive::recursive_schema_types;

#[cfg(test)]
mod tests;
