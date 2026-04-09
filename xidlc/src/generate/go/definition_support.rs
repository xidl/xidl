use crate::generate::go::ParamDirection;
use xidl_parser::hir;

pub(crate) fn param_direction(attr: Option<&hir::ParamAttribute>) -> ParamDirection {
    match attr.map(|value| value.0.as_str()) {
        Some("out") => ParamDirection::Out,
        Some("inout") => ParamDirection::InOut,
        _ => ParamDirection::In,
    }
}

pub(crate) fn is_out_param(attr: Option<&hir::ParamAttribute>) -> bool {
    matches!(param_direction(attr), ParamDirection::Out)
}

pub(crate) fn operation_params(op: &hir::OpDcl) -> &[hir::ParamDcl] {
    op.parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[])
}

pub(crate) fn declarator_name(decl: &hir::Declarator) -> &str {
    match decl {
        hir::Declarator::SimpleDeclarator(value) => &value.0,
        hir::Declarator::ArrayDeclarator(value) => &value.ident,
    }
}
