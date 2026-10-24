mod inheritance;
mod recursive;
mod switch_enums;

use crate::hir::Specification;

fn parse(source: &str) -> Specification {
    let typed = crate::parser::parser_text(source).expect("parse typed ast");
    Specification::from_typed_ast_with_properties(typed, Default::default())
}
