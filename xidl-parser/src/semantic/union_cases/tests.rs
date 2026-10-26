use crate::error::ParserResult;
use crate::hir::Specification;

fn lower(source: &str) -> ParserResult<Specification> {
    Specification::from_typed_ast_with_path(crate::parser::parser_text(source)?, "union-cases.idl")
}

#[test]
fn qualified_enum_labels_remain_as_written_through_typedefs_and_nested_scopes() {
    let spec = lower(
        r#"
        module delivery {
            enum Channel { sms, mms };
            typedef Channel Transport;
            union Payload switch(Transport) {
                case Channel::sms: string text;
                case ::delivery::Channel::mms: string media;
            };
            interface Api {
                enum Status { active, inactive };
                union Result switch(Status) {
                    case Status::active: string value;
                    default: string other;
                };
            };
        };
        "#,
    )
    .expect("explicit enum owners are valid source");
    let serialized = serde_json::to_value(spec).expect("serialize original HIR");
    let labels = &serialized[0]["ModuleDcl"]["definition"][2]["TypeDcl"]["ConstrTypeDcl"]["UnionDef"]
        ["case"];
    assert_eq!(
        labels[0]["label"][0]["Value"]["ScopedName"],
        serde_json::json!({"name": ["Channel", "sms"], "is_root": false})
    );
    assert_eq!(
        labels[1]["label"][0]["Value"]["ScopedName"],
        serde_json::json!({"name": ["delivery", "Channel", "mms"], "is_root": true})
    );
}

#[test]
fn bare_enum_labels_are_rejected_in_every_union_declaration_scope() {
    for (source, path) in [
        (
            "enum Channel { sms }; union Payload switch(Channel) { case sms: string text; };",
            "Payload",
        ),
        (
            "module delivery { enum Channel { sms }; typedef Channel Transport; union Payload switch(Transport) { case sms: string text; }; };",
            "delivery::Payload",
        ),
        (
            "interface Api { enum Channel { sms }; union Payload switch(Channel) { case sms: string text; }; };",
            "Api::Payload",
        ),
        (
            "enum Channel { sms }; typedef union Payload switch(Channel) { case sms: string text; } Alias;",
            "Payload",
        ),
        (
            "enum Channel { sms }; union Outer switch(long) { case 1: union Payload switch(Channel) { case sms: string text; } nested; };",
            "Outer::Payload",
        ),
        (
            "enum Channel { sms, mms }; union Payload switch(Channel) { case Channel::sms: case mms: string text; };",
            "Payload",
        ),
        (
            "enum Channel { sms }; union Payload switch(Channel) { case ::sms: string text; };",
            "Payload",
        ),
    ] {
        let error = lower(source).expect_err("an enum label must name its owner");
        assert!(error.to_string().contains(path), "{error}");
        assert!(error.to_string().contains("Enum::member"), "{error}");
    }
}

#[test]
fn declared_scalar_and_enum_constants_keep_lexical_and_root_bindings() {
    lower(
        r#"
        const long ROOT = 1;
        module delivery {
            const long LIMIT = 2;
            enum Channel { sms };
            const Channel PINNED = Channel::sms;
            typedef long Code;
            union Numeric switch(Code) {
                case LIMIT: string local;
                case ::ROOT: string root;
                case LIMIT + 1: string expression;
                default: string other;
            };
            union Pinned switch(Channel) { case PINNED: string text; };
            module nested {
                union Numeric switch(long) { case LIMIT: string ancestor; };
            };
            interface Api {
                const long LOCAL = 3;
                union Numeric switch(long) { case LOCAL: string local; };
            };
        };
        "#,
    )
    .expect("constant labels are distinct from unqualified enumerators");
}

#[test]
fn constants_in_other_scopes_do_not_authorize_bare_enum_labels() {
    for source in [
        "module other { const long sms = 1; }; enum Channel { sms }; union Payload switch(Channel) { case sms: string text; };",
        "module delivery { const long sms = 1; enum Channel { sms }; union Payload switch(Channel) { case ::sms: string text; }; };",
    ] {
        let error = lower(source).expect_err("the constant must resolve in this lexical scope");
        assert!(error.to_string().contains("case label 'sms'"), "{error}");
    }
}

#[test]
fn literals_and_default_labels_keep_their_existing_contract() {
    lower(
        r#"
        union Numeric switch(long) {
            case -1: string negative;
            case 1: case 2: string positive;
            default: string other;
        };
        union Boolean switch(boolean) { case TRUE: string yes; case FALSE: string no; };
        union Character switch(char) { case 'x': string value; default: string other; };
        "#,
    )
    .expect("literal and default labels require no enum qualification");
}

#[test]
fn source_validation_runs_before_version_projection() {
    let typed = crate::parser::parser_text(
        r#"
        @version enum ApiVersion { v1, v2 };
        enum Channel { sms };
        @removed(ApiVersion::v2)
        union Retired switch(Channel) { case sms: string text; };
        "#,
    )
    .expect("the syntax tree preserves the bare label");
    let error =
        Specification::project_typed_ast_versions(typed, Default::default(), "retired-union.idl")
            .expect_err("retiring a union cannot bypass source validation");
    assert!(error.to_string().contains("union 'Retired'"), "{error}");
    assert!(error.to_string().contains("case label 'sms'"), "{error}");
}
