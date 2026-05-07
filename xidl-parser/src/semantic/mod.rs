mod recursive;
mod recursive_graph;

pub(crate) fn analyze(spec: &mut crate::hir::Specification) {
    recursive::annotate_recursive_members(spec);
}
