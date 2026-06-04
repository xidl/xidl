use super::{format_idl_source, format_jinja_source, format_typescript_source};

#[test]
fn jinja_control_statement_indents_block() {
    let source = "{% if cond %}\nline1\n{% endif %}";
    let formatted = format_jinja_source(source).expect("format jinja");
    assert_eq!(formatted, "{% if cond %}\n    line1\n{% endif %}\n");
}

#[test]
fn jinja_content_braces_indents_block() {
    let source = "{% if cond %}\nfn main() {\nlet x = 1;\n}\n{% endif %}";
    let formatted = format_jinja_source(source).expect("format jinja");
    assert_eq!(
        formatted,
        "{% if cond %}\n    fn main() {\n        let x = 1;\n    }\n{% endif %}\n"
    );
}

#[test]
fn jinja_else_branch_keeps_same_control_depth() {
    let source = "{% if cond %}\nA\n{% else %}\nB\n{% endif %}";
    let formatted = format_jinja_source(source).expect("format jinja");
    assert_eq!(
        formatted,
        "{% if cond %}\n    A\n{% else %}\n    B\n{% endif %}\n"
    );
}

#[test]
fn jinja_set_assignment_does_not_open_block() {
    let source = "{% if cond %}\n{% set ns = namespace(field_id=1) %}\n{% for field in fields %}\nvalue\n{% endfor %}\n{% endif %}";
    let formatted = format_jinja_source(source).expect("format jinja");
    assert_eq!(
        formatted,
        "{% if cond %}\n    {% set ns = namespace(field_id=1) %}\n    {% for field in fields %}\n        value\n    {% endfor %}\n{% endif %}\n"
    );
}

#[test]
fn jinja_skip_comment_in_hash_line_skips_formatting() {
    let source = "## jinja-fmt: skip\n{% if cond %}\nline1\n{% endif %}";
    let formatted = format_jinja_source(source).expect("format jinja");
    assert_eq!(
        formatted,
        "## jinja-fmt: skip\n{% if cond %}\nline1\n{% endif %}"
    );
}

#[test]
fn jinja_skip_comment_in_block_comment_skips_formatting() {
    let source = "{# jinja-fmt: skip #}\n{% if cond %}\nline1\n{% endif %}";
    let formatted = format_jinja_source(source).expect("format jinja");
    assert_eq!(
        formatted,
        "{# jinja-fmt: skip #}\n{% if cond %}\nline1\n{% endif %}"
    );
}

#[test]
fn jinja_skip_only_works_in_header_comments() {
    let source = "{% if cond %}\n## jinja-fmt: skip\nline1\n{% endif %}";
    let formatted = format_jinja_source(source).expect("format jinja");
    assert_eq!(
        formatted,
        "{% if cond %}\n    ## jinja-fmt: skip\n    line1\n{% endif %}\n"
    );
}

#[test]
fn idl_formatter_appends_trailing_newline() {
    let source = "struct Point {\n    int32 x;\n};";
    let formatted = format_idl_source(source).expect("format idl");
    assert!(formatted.ends_with('\n'));
}

#[test]
fn ts_formatter_handles_colon_in_string() {
    let source = "const x = \"event: next\\ndata: \";\n";
    let formatted = format_typescript_source(source).expect("format ts");
    assert_eq!(formatted, "const x = \"event: next\\ndata: \";\n");
}
