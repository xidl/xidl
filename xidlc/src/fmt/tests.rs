use super::format_jinja_source;

#[test]
fn jinja_control_statement_indents_block() {
    let source = "{% if cond %}\nline1\n{% endif %}";
    let formatted = format_jinja_source(source).expect("format jinja");
    assert_eq!(formatted, "{% if cond %}\n    line1\n{% endif %}");
}

#[test]
fn jinja_content_braces_indents_block() {
    let source = "{% if cond %}\nfn main() {\nlet x = 1;\n}\n{% endif %}";
    let formatted = format_jinja_source(source).expect("format jinja");
    assert_eq!(
        formatted,
        "{% if cond %}\n    fn main() {\n        let x = 1;\n    }\n{% endif %}"
    );
}

#[test]
fn jinja_else_branch_keeps_same_control_depth() {
    let source = "{% if cond %}\nA\n{% else %}\nB\n{% endif %}";
    let formatted = format_jinja_source(source).expect("format jinja");
    assert_eq!(
        formatted,
        "{% if cond %}\n    A\n{% else %}\n    B\n{% endif %}"
    );
}
