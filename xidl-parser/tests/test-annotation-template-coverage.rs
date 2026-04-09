use xidl_parser::parser::parser_text;

#[test]
fn parses_raw_custom_and_doc_annotations() {
    let typed = parser_text(
        r#"
        /// doc "quoted"
        @foo::bar(key=1)
        @data-representation([XCDR2])
        struct Item {
            /// field docs
            @id(7)
            long value;
        };

        bitmask Flags {
            /// flag docs
            @key ready
        };
        "#,
    )
    .expect("parse should succeed");

    let debug = format!("{typed:#?}");
    assert!(debug.contains("doc"));
    assert!(debug.contains("quoted"));
    assert!(debug.contains("ScopedName"));
    assert!(debug.contains("data-representation"));
    assert!(debug.contains("ready"));
}

#[test]
fn parses_template_module_parameter_variants() {
    let typed = parser_text(
        r#"
        module demo {
            module Holder <typename T, interface I, sequence S, long N, string Name, boolean Flag> {
                typedef T Value;
            };
        };
        "#,
    )
    .expect("parse should succeed");

    let debug = format!("{typed:#?}");
    assert!(debug.contains("TemplateModuleDcl"));
    assert!(debug.contains("Typename"));
    assert!(debug.contains("Interface"));
    assert!(debug.contains("SequenceKeyword"));
    assert!(debug.contains("FormalParameter"));
    assert!(debug.contains("Value"));
}

#[test]
fn parses_preproc_define_call_and_include_variants() {
    let typed = parser_text(
        r#"
        #define FOO bar
        #pragma xidlc package demo.pkg
        #include "local.idl"
        #include <system.idl>
        "#,
    )
    .expect("parse should succeed");

    let debug = format!("{typed:#?}");
    assert!(debug.contains("PreprocDefine"));
    assert!(debug.contains("PreprocCall"));
    assert!(debug.contains("StringLiteral"));
    assert!(debug.contains("SystemLibString"));
}
