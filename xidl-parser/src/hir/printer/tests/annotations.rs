use super::*;

#[test]
fn render_annotations_covers_builtin_and_parameter_shapes() {
    let version = scoped(&["Versions", "v2"], false);
    let annotations = [
        Annotation::Id { value: "7".into() },
        Annotation::Key {
            value: Some("true".into()),
        },
        Annotation::AutoId { value: None },
        Annotation::Optional {
            value: Some("false".into()),
        },
        Annotation::Position { value: "2".into() },
        Annotation::Value { value: "3".into() },
        Annotation::Extensibility {
            kind: "FINAL".into(),
        },
        Annotation::Final,
        Annotation::Appendable,
        Annotation::Mutable,
        Annotation::MustUnderstand { value: None },
        Annotation::Default { value: "0".into() },
        Annotation::Range {
            min: "0".into(),
            max: "9".into(),
        },
        Annotation::Min { value: "0".into() },
        Annotation::Max { value: "9".into() },
        Annotation::Unit { value: "ms".into() },
        Annotation::BitBound { value: "8".into() },
        Annotation::External { value: None },
        Annotation::Nested {
            value: Some("true".into()),
        },
        Annotation::Verbatim {
            language: Some("c".into()),
            placement: Some("BEGIN_FILE".into()),
            text: "\"body\"".into(),
        },
        Annotation::Service {
            platform: Some("DDS".into()),
        },
        Annotation::Oneway { value: None },
        Annotation::Ami {
            value: Some("true".into()),
        },
        Annotation::HashId { value: None },
        Annotation::DefaultNested {
            value: Some("true".into()),
        },
        Annotation::IgnoreLiteralNames { value: None },
        Annotation::TryConstruct {
            value: Some("discard".into()),
        },
        Annotation::NonSerialized { value: None },
        Annotation::DataRepresentation {
            kinds: vec!["XCDR1".into(), "XCDR2".into()],
        },
        Annotation::Topic {
            name: Some("events".into()),
            platform: Some("DDS".into()),
        },
        Annotation::Choice,
        Annotation::Empty,
        Annotation::DdsService,
        Annotation::DdsRequestTopic {
            name: "request".into(),
        },
        Annotation::DdsReplyTopic {
            name: "reply".into(),
        },
        Annotation::DefaultLiteral,
        Annotation::Rename {
            name: "wire_name".into(),
        },
        Annotation::RenameAll {
            rule: RenameRule::CamelCase,
        },
        Annotation::Skip,
        Annotation::Version,
        Annotation::VersionParameter(VersionParameter {
            location: HttpParameterLocation::Query,
            name: "api_version".into(),
        }),
        Annotation::Added {
            version: version.clone(),
        },
        Annotation::Removed { version },
    ];
    let rendered = annotations
        .iter()
        .map(IdlPrinter::render_annotation)
        .collect::<Vec<_>>();
    for expected in [
        "@id(7)",
        "@key(true)",
        "@autoid",
        "@optional(false)",
        "@position(2)",
        "@value(3)",
        "@extensibility(FINAL)",
        "@final",
        "@appendable",
        "@mutable",
        "@must_understand",
        "@default(0)",
        "@range(min = 0, max = 9)",
        "@min(0)",
        "@max(9)",
        "@unit(ms)",
        "@bit_bound(8)",
        "@external",
        "@nested(true)",
        "@verbatim(C, BEGIN_FILE, \"body\")",
        "@service(DDS)",
        "@oneway",
        "@ami(true)",
        "@hashid",
        "@default_nested(true)",
        "@ignore_literal_names",
        "@try_construct(discard)",
        "@non_serialized",
        "@data_representation(XCDR1 | XCDR2)",
        "@topic(events, DDS)",
        "@choice",
        "@empty",
        "@dds_service",
        "@dds_request_topic(request)",
        "@dds_reply_topic(reply)",
        "@default_literal",
        "@rename(\"wire_name\")",
        "@rename_all(\"camelCase\")",
        "@skip",
        "@version",
        "@version(in = \"query\", name = \"api_version\")",
        "@added(Versions::v2)",
        "@removed(Versions::v2)",
    ] {
        assert!(rendered.iter().any(|value| value == expected), "{expected}");
    }

    let params = [
        None,
        Some(AnnotationParams::ConstExpr(integer("1"))),
        Some(AnnotationParams::Positional(vec![
            integer("1"),
            integer("2"),
        ])),
        Some(AnnotationParams::Params(vec![
            AnnotationParam {
                ident: "path".into(),
                value: Some(ConstExpr::Literal(Literal::StringLiteral("/items".into()))),
            },
            AnnotationParam {
                ident: "secure".into(),
                value: None,
            },
        ])),
        Some(AnnotationParams::Raw("raw = 1".into())),
    ];
    for (index, params) in params.into_iter().enumerate() {
        let name = format!("custom_{index}");
        assert_eq!(
            IdlPrinter::render_annotation(&Annotation::Builtin {
                name: name.clone(),
                params
            }),
            match index {
                0 => format!("@{name}"),
                1 => format!("@{name}(1)"),
                2 => format!("@{name}(1, 2)"),
                3 => format!("@{name}(path = \"/items\", secure)"),
                _ => format!("@{name}(raw=1)"),
            }
        );
    }
    assert_eq!(
        IdlPrinter::render_annotation(&Annotation::ScopedName {
            name: scoped(&["pkg", "audit"], true),
            params: Some(AnnotationParams::Raw("enabled".into())),
        }),
        "@::pkg::audit(enabled)"
    );
    assert_eq!(IdlPrinter::render_optional_value("flag", None), "@flag");
    assert_eq!(
        IdlPrinter::render_optional_value("flag", Some("true")),
        "@flag(true)"
    );
    assert_eq!(
        IdlPrinter::render_annotations_inline(&[named_annotation("doc"), named_annotation("get")]),
        "@get "
    );
}
