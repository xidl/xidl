use xidl_parser::hir;

pub(crate) fn render_annotation_const_expr(expr: &hir::ConstExpr) -> String {
    super::render_const(expr)
}
