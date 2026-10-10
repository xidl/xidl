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
fn builtin_stays_out_of_user_models_and_survives_hir_round_trip() {
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
    assert!(
        enums.is_empty(),
        "compiler declarations are not generated models"
    );
    assert_eq!(spec.0.len(), 1, "only the user module is retained");
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
fn rejects_runtime_references_to_compile_time_builtins() {
    for declaration in [
        "struct Meta { ::ContentType media; };",
        "struct Meta { sequence<ContentType> media; };",
        "struct Meta { map<string, ContentType> media; };",
        "struct Meta { optional<ContentType> media; };",
        "struct Meta { string media[ContentType::Json]; };",
        "struct Meta { string<ContentType::Json> media; };",
        "struct Meta : ContentType { string value; };",
        "typedef ContentType Media;",
        "const ContentType media = ContentType::Json;",
        "const long media = ContentType::Json;",
        "exception Failure { ContentType media; };",
        "interface Api { ContentType read(); };",
        "interface Api { void write(in ContentType media); };",
        "interface Api { readonly attribute ContentType media; };",
        "interface Api { attribute ContentType media; };",
        "interface Api { void read() raises(ContentType); };",
        "interface Api { typedef ContentType Media; };",
        "interface Api { exception Failure { ContentType media; }; };",
        "union Ordinary switch(ContentType) { case Json: string value; };",
        "union Ordinary switch(long) { case ContentType::Json: string value; };",
        "@http union Other switch(ContentType) { case Json: ContentType value; };",
    ] {
        let source = format!(
            "@http union Payload switch(ContentType) {{ case Json: string value; }}; {declaration}"
        );
        let error = lower(&source).expect_err("builtin has no runtime representation");
        assert!(
            error.to_string().contains("compile-time only"),
            "{declaration}: {error}"
        );
    }
}

#[test]
fn runtime_types_can_shadow_the_builtin_in_modules_and_interfaces() {
    let spec = lower(
        r#"
        @http union Payload switch(ContentType) { case Json: string value; };
        module models {
            enum ContentType { Custom, };
            struct Meta { ContentType media; };
            typedef ContentType Media;
            const ContentType media = ContentType::Custom;
            union Ordinary switch(ContentType) { case Custom: string value; };
        };
        interface Api {
            enum ContentType { Local, };
            ContentType read(in ContentType value);
            attribute ContentType media;
        };
        "#,
    )
    .expect("user runtime types keep their lexical bindings");
    assert_eq!(spec.0.len(), 3);
    crate::rest_hir::project(&spec).expect("projection retains the same lexical bindings");
}

#[test]
fn interface_expansion_preserves_source_scope_validation() {
    let source = r#"
        @http union Payload switch(ContentType) { case Json: string value; };
        interface Api {
            enum ContentType { Custom, };
            ContentType read(in ContentType value);
            attribute ContentType media;
        };
    "#;
    let typed = crate::parser::parser_text(source).expect("valid source");
    Specification::from_typed_ast_with_path(typed, "interface-local.idl")
        .expect("source scope is checked before generated wrappers are added");
    for expand in [false, true] {
        Specification::project_typed_ast_with_properties_and_path(
            crate::parser::parser_text(source).expect("valid source"),
            HashMap::from([
                ("hir_kind".to_string(), "http".into()),
                ("expand_interface".to_string(), expand.into()),
            ]),
            "interface-local.idl",
        )
        .expect(
            "projection collects representations without rechecking generated wrappers as source",
        );
    }
}

#[test]
fn rejects_http_unions_inside_interfaces_before_transport_projection() {
    let error = lower(
        "interface Api { @http union Payload switch(ContentType) { case Text: string value; }; Payload read(); };"
    ).expect_err("interface-local models are not supported by HTTP generators");
    assert!(
        error
            .to_string()
            .contains("declare it at module or root scope"),
        "{error}"
    );
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
