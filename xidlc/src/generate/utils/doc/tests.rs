use super::doc_lines_from_annotations;
use xidl_parser::{hir, parser::parser_text};

fn annotations(source: &str) -> Vec<hir::Annotation> {
    let specification = hir::Specification::from(parser_text(source).unwrap());
    let hir::Definition::TypeDcl(hir::TypeDcl::ConstrTypeDcl(hir::ConstrTypeDcl::StructDcl(
        declaration,
    ))) = &specification.0[0]
    else {
        panic!("documentation fixture must declare a struct");
    };
    declaration.annotations.clone()
}

#[test]
fn documentation_comments_preserve_markdown_unicode_and_literal_escapes() {
    let source = r#"
/// 文档 💳 "quoted".
///
/// Use **Markdown**, `\u0041`, and \_literal.
///
/// - Keep a stable identifier.
/// - Read the selected version.
///
/// ```text
/// left<TAB>right
/// ```
struct Invoice { string id; };
"#
    .replace("<TAB>", "\t");
    assert_eq!(
        doc_lines_from_annotations(&annotations(&source)).join("\n"),
        concat!(
            "文档 💳 \"quoted\".\n\n",
            "Use **Markdown**, `\\u0041`, and \\_literal.\n\n",
            "- Keep a stable identifier.\n- Read the selected version.\n\n",
            "```text\nleft\tright\n```"
        )
    );
}

#[test]
fn source_annotations_are_custom_metadata_and_cannot_replace_doc_comments() {
    for annotation in [
        r#"@doc("Annotation text")"#,
        r#"@doc(text = "Annotation text")"#,
        r#"@doc(L"Annotation text")"#,
        r#"@vendor::doc("Vendor metadata")"#,
        r#"@doc("first", ["array"], "last")"#,
    ] {
        let declaration = format!("{annotation}\nstruct Invoice {{ string id; }};");
        assert!(doc_lines_from_annotations(&annotations(&declaration)).is_empty());
        let documented = format!("/// Actual **Markdown** documentation.\n{declaration}");
        assert_eq!(
            doc_lines_from_annotations(&annotations(&documented)),
            ["Actual **Markdown** documentation."]
        );
    }
}
