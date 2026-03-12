use super::*;
use xidl_parser::hir;

#[test]
fn test_parse_timestamp() {
    let timestamp = [
        //
        0,
        1000,
        i64::MAX,
        i64::MIN,
    ];

    for case in timestamp {
        let _ = format_timestamp_filter(case);
    }
}

#[test]
fn test_doc_lines_from_raw() {
    let annotations = vec![hir::Annotation::Builtin {
        name: "doc".to_string(),
        params: Some(hir::AnnotationParams::Raw(
            "\"hello\\\\nworld\"".to_string(),
        )),
    }];
    let doc = doc_lines_from_annotations(&annotations);
    assert_eq!(doc, vec!["hello", "world"]);
}

#[test]
fn test_doc_lines_from_const_expr() {
    let expr = hir::ConstExpr(hir::OrExpr::XorExpr(hir::XorExpr::AndExpr(
        hir::AndExpr::ShiftExpr(hir::ShiftExpr::AddExpr(hir::AddExpr::MultExpr(
            hir::MultExpr::UnaryExpr(hir::UnaryExpr::PrimaryExpr(hir::PrimaryExpr::Literal(
                hir::Literal::StringLiteral("hi".to_string()),
            ))),
        ))),
    )));
    let annotations = vec![hir::Annotation::Builtin {
        name: "doc".to_string(),
        params: Some(hir::AnnotationParams::ConstExpr(expr)),
    }];
    let doc = doc_lines_from_annotations(&annotations);
    assert_eq!(doc, vec!["hi"]);
}
