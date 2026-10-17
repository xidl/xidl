use super::*;

fn scoped(parts: &[&str], is_root: bool) -> ScopedName {
    ScopedName {
        name: parts.iter().map(|part| (*part).to_string()).collect(),
        is_root,
    }
}

fn integer(value: &str) -> ConstExpr {
    ConstExpr::Literal(Literal::IntegerLiteral(IntegerLiteral(value.to_string())))
}

fn positive(value: &str) -> PositiveIntConst {
    PositiveIntConst(integer(value))
}

fn named_annotation(name: &str) -> Annotation {
    Annotation::Builtin {
        name: name.to_string(),
        params: None,
    }
}

fn lower(source: &str) -> Specification {
    Specification::from_typed_ast_with_properties(
        crate::parser::parser_text(source).expect("parse source"),
        [("expand_interface".to_string(), serde_json::json!(false))]
            .into_iter()
            .collect(),
    )
}

mod annotations;
mod render;
mod roundtrip;
