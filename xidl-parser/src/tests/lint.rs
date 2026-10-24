use crate::lint::{DiagnosticCode, Severity, lint};
use crate::parser::{normalize_source_for_tree_sitter, parser_text};

#[test]
fn annotation_diagnostics_are_source_located_and_stable() {
    let source = r#"// 中文 😀
@version enum V { v1, v2 };
struct Item {
    @remvoed(V::v2) string legacy;
    @relay_audit string id;
};
@optional interface Items {
    @gett(path = "/items") Item get_item();
    @api_key(in = "body", name = "key") void save();
};
"#;
    let ast = parser_text(source).unwrap();
    let diagnostics = lint(&ast);
    assert_eq!(diagnostics, lint(&ast));
    assert_eq!(diagnostics.len(), 5);
    assert_eq!(&source[diagnostics[0].span.clone()], "@remvoed(V::v2)");
    assert_eq!(diagnostics[0].suggestion.as_deref(), Some("removed"));
    assert_eq!(diagnostics[1].severity, Severity::Warning);
    assert_eq!(diagnostics[2].code, DiagnosticCode::MisplacedAnnotation);
    assert_eq!(diagnostics[3].suggestion.as_deref(), Some("get"));
    assert_eq!(diagnostics[4].code, DiagnosticCode::BadAnnotationArgs);
    insta::assert_snapshot!(serde_json::to_string_pretty(&diagnostics).unwrap());
}

#[test]
fn lint_accepts_real_http_version_security_and_stream_contracts() {
    let source = r#"
@version(in="QuErY", name="api_version") enum V { @rename("2026-01") v1, v2 };
@http(404) exception Missing { @header("X-Reason") string reason; @body string message; };
struct Details { string name; };
@derive(Debug, Clone) struct Item { @flatten Details details; @optional string note; };
@api_key(in="HEADER", name="X-Key")
@cors("https://one.example", "https://two.example") interface Items {
    @get(path="/items/{id}") Item get_item(@path string id) raises(Missing);
    @deprecated(since="2025-01-01", after="2027-01-01") void old();
    @upgrade(protocol="websocket", codec="json", heartbeat="1s", max_message=1024)
    void watch();
    @server_stream readonly attribute string status;
};
"#;
    assert!(lint(&parser_text(source).unwrap()).is_empty());
}

#[test]
fn argument_errors_preserve_duplicates_types_and_raw_boundaries() {
    let declarations = [
        "@version(in=\"header\") enum V { v1 };",
        "@version(name=\"v\") enum V { v1 };",
        "@version(in=\"body\",name=\"v\") enum V { v1 };",
        "@version(in=\"query\",name=\"\") enum V { v1 };",
        "@version(in=\"query\",name=3) enum V { v1 };",
        "@version(in=\"query\",name=\"v\",extra=1) enum V { v1 };",
        "@api_key(in=\"body\", IN=\"header\", name=\"X[Key]\") interface I {};",
        "@api_key(in=\"header\") interface I {};",
        "@rename(1) struct S { string x; };",
        "@rename(\"a[0]\", value=\"b\") struct S { string x; };",
        "@added(\"v1\") struct S { string x; };",
        "interface I { @get(\"/a[0]\", \"/b[0]\") void f(); };",
        "interface I { @get(path=\"/x\", PATH=\"/y\") void f(); };",
        "interface I { @upgrade(protocol=\"websocket\",codec=\"unknown\") void f(); };",
        "interface I { @upgrade(protocol=\"websocket\",heartbeat=\"oops\") void f(); };",
        "@cors(origin=\"https://example.com\") interface I {};",
    ];
    for source in declarations {
        let diagnostics = lint(&parser_text(source).unwrap());
        assert!(
            diagnostics
                .iter()
                .any(|item| item.code == DiagnosticCode::BadAnnotationArgs),
            "missing argument diagnostic for {source}: {diagnostics:?}"
        );
    }
}

#[test]
fn extended_annotations_report_once_and_passthroughs_remain_opaque() {
    let source = r#"//@relay_audit
struct Item {
    //@remvoed(V::v2)
    string old;
    @RuSt_derive(Debug) @TS-special("opaque[0]") string id;
};
"#;
    let diagnostics = lint(&parser_text(source).unwrap());
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    assert_eq!(&source[diagnostics[0].span.clone()], "//@relay_audit");
    assert_eq!(&source[diagnostics[1].span.clone()], "//@remvoed(V::v2)");
}

#[test]
fn nested_declarations_are_checked_without_lowering_or_type_resolution() {
    let source = r#"
module Api {
    @version enum V { v1, @remvoed(V::v2) v2 };
    @version struct Invalid { string value; };
    interface I {
        struct Nested { @gett string x; };
        @http(404) exception Missing { @relay_audit string reason; };
        void f(@optional @get string id);
    };
    union Result switch(long) { case 1: @optional @post string value; };
};
"#;
    let diagnostics = lint(&parser_text(source).unwrap());
    assert_eq!(diagnostics.len(), 6, "{diagnostics:?}");
    assert!(
        diagnostics
            .windows(2)
            .all(|pair| pair[0].span.start < pair[1].span.start)
    );
}

#[test]
fn normalization_preserves_unicode_comments_strings_and_scoped_names() {
    let source = "// 中文 😀 \" @OPTIONAL\r\n/* @VERSION */\r\n@ VERSION enum V { v1 };\r\nstruct S { @ /* note */ OPTIONAL string text; @rust-derive(Debug) string other; };\r\nconst string Doc = \"@OPTIONAL 中文\";\r\n";
    let normalized = normalize_source_for_tree_sitter(source);
    assert_eq!(source.len(), normalized.len());
    assert!(normalized.contains("// 中文 😀 \" @OPTIONAL\r\n"));
    assert!(normalized.contains("/* @VERSION */"));
    assert!(normalized.contains("@ /* note */ optional"));
    assert!(normalized.contains("\"@OPTIONAL 中文\""));
    assert!(lint(&parser_text(source).unwrap()).is_empty());
    let scoped = "@Vendor::GET struct S { string id; };";
    assert_eq!(normalize_source_for_tree_sitter(scoped), scoped);
}

#[test]
fn extended_raw_arguments_do_not_absorb_adjacent_annotations() {
    for annotations in [
        r#"//@id(11) @relay_audit(data=["x"])"#,
        r#"//@rename("new") @relay_audit(data=["x"])"#,
        r#"//@rename("new[0]") @id(11) @relay_audit(data=["x"])"#,
    ] {
        let source = format!("struct S {{ long a; {annotations}\n short b; }};");
        let diagnostics = lint(&parser_text(&source).unwrap());
        assert_eq!(diagnostics.len(), 1, "{source}: {diagnostics:?}");
        assert_eq!(diagnostics[0].code, DiagnosticCode::UnknownAnnotation);
        assert_eq!(
            &source[diagnostics[0].span.clone()],
            r#"@relay_audit(data=["x"])"#
        );
    }
}

#[test]
fn annotation_comment_trivia_cannot_change_argument_shape() {
    for comment in ["note", "[", ") [", "= , \"", "中文 😀"] {
        let source = format!(
            "struct S {{ @id(/* {comment} */ 1) long x; @rename(/* {comment} */ \"new[0]\") long y; }};"
        );
        let ast = parser_text(&source).unwrap();
        assert!(lint(&ast).is_empty(), "{source}: {:?}", lint(&ast));
    }
}

#[test]
fn editor_diagnostics_cover_nested_and_forward_declarations_without_resolving_includes() {
    let source = r#"
#include "editor-may-not-have-this.idl"
#define ENABLED 1
@relay_audit native Handle;
const long LIMIT = 4;
@relay_audit struct Forward;
@relay_audit union Future;
@relay_audit bitset Bits { bitfield<1> enabled; };
@relay_audit bitmask Flags { @relay_audit Read, Write };
@relay_audit typedef long Count;
@relay_audit typedef struct Inline { @relay_audit long id; } Alias;
union Choice switch(long) { case 1: struct Nested { @relay_audit string id; } item; };
@relay_audit interface Service;
@relay_audit interface Empty {};
interface Api {
    const long COUNT = 4;
    @relay_audit attribute string label;
};
module Generic<typename T> {
    @relay_audit struct Item { T value; };
};
"#;
    let ast = parser_text(source).unwrap();
    let diagnostics = lint(&ast);
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.span.start)
            .collect::<Vec<_>>(),
        source
            .match_indices("@relay_audit")
            .map(|(start, _)| start)
            .collect::<Vec<_>>(),
    );
    for diagnostic in diagnostics {
        assert_eq!(diagnostic.code.as_str(), "unknown-annotation");
        assert_eq!(diagnostic.severity, Severity::Warning);
        assert_eq!(&source[diagnostic.span], "@relay_audit");
    }
    for (source, code) in [
        ("@optional interface I {};", "misplaced-annotation"),
        (
            "interface I { @gett void read(); };",
            "misspelled-annotation",
        ),
        (
            "interface I { @get(other=1) void read(); };",
            "bad-annotation-args",
        ),
    ] {
        assert_eq!(lint(&parser_text(source).unwrap())[0].code.as_str(), code);
    }
}

#[test]
fn annotation_argument_policy_rejects_ambiguous_and_incomplete_contracts() {
    for annotation in [
        "@derive",
        "@get(other=\"route\")",
        "@get(\"x\", other=\"y\")",
        "@get(\"x[0]\", other=\"y\")",
        "@get(path=\"x[0]\",path=\"y\")",
        "@deprecated(\"not-a-date\")",
        "@upgrade(protocol=\"fastnet\", codec=\"json\")",
        "@upgrade(protocol=\"websocket\", max_message=0)",
        "@upgrade(protocol=\"websocket\", max_message=\"many\")",
    ] {
        let source = format!("interface I {{ {annotation} void read(); }};");
        let diagnostics = lint(&parser_text(&source).unwrap());
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::BadAnnotationArgs),
            "{annotation}: {diagnostics:?}"
        );
    }
}
