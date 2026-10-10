use crate::error::ParserResult;
use crate::hir::{self, Specification};
use std::collections::HashMap;

fn lower(source: &str) -> ParserResult<Specification> {
    Specification::from_typed_ast_with_properties_and_path(
        crate::parser::parser_text(source)?,
        HashMap::from([("hir_kind".to_string(), "http".into())]),
        "builtin-test.idl",
    )
}

#[test]
fn loads_one_builtin_for_nested_unions_and_resolves_qualified_cases() {
    let spec = lower(
        r#"
        module api {
            @http union First switch(ContentType) { case Json: string value; };
            @http union Second switch(::ContentType) {
                case ::ContentType::OctetStream: sequence<octet> bytes;
                case ContentType::Text: string text;
            };
        };
        "#,
    )
    .expect("implicit builtin");
    let enums = spec
        .0
        .iter()
        .filter_map(|definition| match definition {
            hir::Definition::TypeDcl(hir::TypeDcl::ConstrTypeDcl(hir::ConstrTypeDcl::EnumDcl(
                e,
            ))) => Some(e),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(enums.len(), 1);
    assert_eq!(enums[0].ident, "ContentType");
    let serialized = serde_json::to_string(&spec).expect("serialize HIR");
    let restored: Specification = serde_json::from_str(&serialized).expect("restore HIR");
    let document = crate::rest_hir::project(&restored).expect("project restored HIR");
    let cases = document
        .document
        .http_unions
        .iter()
        .flat_map(|u| &u.cases)
        .map(|case| case.content_type.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        cases,
        ["application/json", "application/octet-stream", "text/plain"]
    );
    assert_eq!(serde_json::to_string(&document.spec).unwrap(), serialized);
}

#[test]
fn ordinary_unions_do_not_load_or_reserve_content_type() {
    let spec = lower("enum ContentType { Custom, }; union Ordinary switch(ContentType) { case Custom: string value; default: long other; };")
        .expect("ordinary union rules unchanged");
    assert_eq!(spec.0.len(), 2);
    let doc = crate::rest_hir::project(&spec).expect("ordinary union");
    assert!(doc.document.http_unions.is_empty());
}

#[test]
fn rejects_user_root_content_type_declarations() {
    for declaration in [
        "enum ContentType { Json, };",
        "struct ContentType { string value; };",
        "typedef long ContentType;",
        "typedef enum ContentType { Json, } Alias;",
        "native ContentType;",
        "module ContentType { struct Value { string value; }; };",
        "const long ContentType = 1;",
        "exception ContentType { string value; };",
        "interface ContentType {};",
    ] {
        let source = format!(
            "{declaration} @http union Payload switch(::ContentType) {{ case Json: string value; }};"
        );
        let error = lower(&source).expect_err("reserved root declaration");
        assert!(
            error.to_string().contains("built-in root declaration"),
            "{declaration}: {error}"
        );
    }
}

#[test]
fn local_shadow_requires_explicit_root_discriminator() {
    for declaration in [
        "enum ContentType { Json, };",
        "struct ContentType { string value; };",
        "typedef long ContentType;",
        "typedef enum ContentType { Json, } Alias;",
        "native ContentType;",
        "module ContentType { struct Value { string value; }; };",
        "const long ContentType = 1;",
        "exception ContentType { string value; };",
        "interface ContentType {};",
    ] {
        let source = format!(
            "module api {{ {declaration} @http union Payload switch(ContentType) {{ case Json: string value; }}; }};"
        );
        let error = lower(&source).expect_err("local declaration shadows builtin");
        assert!(
            error
                .to_string()
                .contains("must resolve to the built-in ::ContentType"),
            "{declaration}: {error}"
        );
        lower(&source.replace("switch(ContentType)", "switch(::ContentType)"))
            .expect("explicit root discriminator bypasses local shadow");
    }
}

#[test]
fn http_unions_reject_custom_discriminators_and_open_or_ambiguous_cases() {
    for (source, expected) in [
        (
            "enum Mime { Json, }; @http union Payload switch(Mime) { case Json: string value; };",
            "must resolve to the built-in ::ContentType",
        ),
        (
            "@http union Payload switch(long) { case 0: string value; };",
            "must be the built-in ::ContentType",
        ),
        (
            "@http union Payload switch(ContentType) { default: string value; };",
            "no default",
        ),
        (
            "@http union Payload switch(ContentType) { case Json: case Text: string value; };",
            "exactly one",
        ),
        (
            "@http union Payload switch(ContentType) { case Json: string a; case ::ContentType::Json: string b; };",
            "duplicate ContentType case 'Json'",
        ),
        (
            "@http union Payload switch(ContentType) { case Missing: string value; };",
            "must name a member",
        ),
        (
            "enum Other { Json, }; @http union Payload switch(ContentType) { case Other::Json: string value; };",
            "must name a member",
        ),
        (
            "@http union Payload switch(ContentType) { case 1: string value; };",
            "case labels must name enum members",
        ),
        (
            "module api { enum ContentType { Json, }; @http union Payload switch(::ContentType) { case ContentType::Json: string value; }; };",
            "must name a member",
        ),
    ] {
        let error = lower(source).expect_err("invalid unused HTTP union declaration");
        assert!(error.to_string().contains(expected), "{source}: {error}");
    }
}
