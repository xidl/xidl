use crate::{hir, parser::parser_text};

#[test]
fn compiler_http_types_cannot_escape_into_user_values_bounds_or_exceptions() {
    for declaration in [
        "struct Meta { wstring<ContentType::Json> text; };",
        "struct Meta { fixed<ContentType::Json,2> amount; };",
        "struct Meta { fixed<8,ContentType::Json> amount; };",
        "struct Meta { long value default ContentType::Json; };",
        "const sequence<ContentType> values = 0;",
        "const string<ContentType::Json> text = \"\";",
        "const wstring<ContentType::Json> text = L\"\";",
        "bitset Bits : ContentType { bitfield<1> bit; };",
        "bitset Bits { bitfield<ContentType::Json> bit; };",
        "interface Api : ContentType {};",
        "interface Api { const long value = ContentType::Json; };",
        "interface Api { attribute string text getraises(ContentType); };",
        "exception Missing { string text; }; interface Api { attribute string text getraises(Missing) setraises(ContentType); };",
        "interface Api { attribute string text setraises(ContentType); };",
    ] {
        let source = format!(
            "@http union Payload switch(ContentType) {{ case ContentType::Json: string value; }}; {declaration}"
        );
        let ast = parser_text(&source).unwrap_or_else(|error| panic!("{declaration}: {error}"));
        let error = hir::Specification::from_typed_ast_with_properties_and_path(
            ast,
            [("hir_kind".into(), "http".into())].into(),
            "contract.idl",
        )
        .expect_err("runtime-only type cannot be emitted as user data");
        let message = error.to_string();
        assert!(
            message.contains("ordinary IDL values cannot reference runtime types"),
            "{declaration}: {message}"
        );
        assert!(message.contains("ContentType"), "{message}");
    }
}
