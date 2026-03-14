const TEST_CASES: &[(&str, &str)] = &[
    (
        "annotation_basic",
        r#"
        @id(1)
        struct S {
            @id(10) long a; //@id(11)
            @optional short b;
        };

        @my::anno(abc=1)
        enum E { @id(0) A, @id(1) B };

        @id(2)
        bitmask BM { @id(0) A, @id(1) B };

        @id(3)
        union U switch (long) {
            case 0: @id(100) long a;
            default: long b;
        };

        @id(4)
        interface I {
            @oneway void ping();
            @key attribute long value;
        };
    "#,
    ),
    (
        "annotation_http_rfc",
        r#"
        @http-basic
        @api-key(in = "header", name = "X-API-Key")
        interface HttpApi {
            @deprecated
            void old_ping();

            @deprecated("2026-03-13")
            @http-bearer
            string get_user(
                @query string id
            );

            @deprecated(since = "2026-03-13", after = "2026-03-14T12:00:00+08:00")
            @oauth2(scopes = ["read:users", "write:users"])
            string update_user(
                @header("X-Req-Id") string request_id,
                string name
            );

            @no-security
            string health();
        };
    "#,
    ),
];

#[test]
fn test_typed_ast() {
    for (name, text) in TEST_CASES {
        let ast = xidl_parser::parser::parser_text(text).unwrap();
        let snapshot = format!("typed_ast__{name}");
        insta::assert_debug_snapshot!(snapshot, ast);
    }
}

#[test]
fn test_hir() {
    for (name, text) in TEST_CASES {
        let typed = xidl_parser::parser::parser_text(text).unwrap();
        let hir = xidl_parser::hir::Specification::from(typed);
        let snapshot = format!("hir__{name}");
        insta::assert_debug_snapshot!(snapshot, hir);
    }
}
