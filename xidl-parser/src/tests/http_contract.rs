use crate::{hir, parser::parser_text, rest_hir::semantics::*};

fn annotations(source: &str) -> Vec<hir::Annotation> {
    let spec = hir::Specification::from_typed_ast_with_properties_and_path(
        parser_text(&format!("{source} interface Api {{}};")).unwrap(),
        [("expand_interface".into(), false.into())].into(),
        "security.idl",
    )
    .unwrap();
    let hir::Definition::InterfaceDcl(interface) = &spec.0[0] else {
        panic!("interface")
    };
    interface.annotations.clone()
}

#[test]
fn explicit_security_overrides_and_inheritance_agree_for_both_public_views() {
    for (interface, method, expected, origin) in [
        ("", "", None, None),
        (
            "@http_basic",
            "",
            Some(vec![HttpSecurityRequirement::HttpBasic]),
            Some(HttpSecurityOrigin::Interface),
        ),
        (
            "@http_basic",
            "@no_security",
            Some(vec![]),
            Some(HttpSecurityOrigin::Method),
        ),
        (
            "@no_security",
            "",
            Some(vec![]),
            Some(HttpSecurityOrigin::Interface),
        ),
        (
            "@http_basic",
            "@http_bearer",
            Some(vec![HttpSecurityRequirement::HttpBearer]),
            Some(HttpSecurityOrigin::Method),
        ),
    ] {
        let interface = annotations(interface);
        let method = annotations(method);
        assert_eq!(effective_security(&interface, &method).unwrap(), expected);
        let located = effective_security_with_origin(&interface, &method).unwrap();
        assert_eq!(located.as_ref().map(|profile| profile.origin), origin);
        assert_eq!(located.map(|profile| profile.requirements), expected);
    }
    for invalid in [
        "@api_key(in=\"body\", name=\"token\")",
        "@http_basic @no_security",
    ] {
        let error = effective_security(&annotations(invalid), &[]).unwrap_err();
        assert!(
            error.to_string().contains(if invalid.starts_with("@api") {
                "header|query|cookie"
            } else {
                "cannot be combined"
            }),
            "{error}"
        );
    }
}

#[test]
fn websocket_options_accept_real_codecs_and_reject_invalid_handshake_contracts() {
    for (codec, expected) in [
        ("json", WebSocketCodec::Json),
        ("msgpack", WebSocketCodec::Msgpack),
        ("bytes", WebSocketCodec::Bytes),
    ] {
        let values = annotations(&format!(
            r#"@upgrade(protocol="websocket",codec="{codec}",subprotocol="events.v1",heartbeat="20s",max_message=4096)"#
        ));
        assert_eq!(
            parse_upgrade_protocol(&values).unwrap().as_deref(),
            Some("websocket")
        );
        let config = parse_websocket_config(&values, UpgradeMode::WebSocket)
            .unwrap()
            .unwrap();
        assert_eq!(config.codec, expected);
        assert_eq!(config.subprotocol.as_deref(), Some("events.v1"));
        assert_eq!(config.heartbeat_ms, Some(20_000));
        assert_eq!(config.max_message_bytes, Some(4096));
    }
    for (argument, fragment) in [
        ("codec=\"unknown\"", "unsupported"),
        ("subprotocol=\"bad token\"", "RFC 6455"),
        ("heartbeat=\"\"", "cannot be empty"),
        ("max_message=\"many\"", "positive integer"),
        ("max_message=0", ">= 1"),
        ("unknown=1", "unknown parameter"),
    ] {
        let values = annotations(&format!("@upgrade(protocol=\"websocket\",{argument})"));
        let error = parse_websocket_config(&values, UpgradeMode::WebSocket).unwrap_err();
        assert!(error.contains(fragment), "{argument}: {error}");
    }
    for (argument, fragment) in [
        ("codec=\"json\"", "only valid"),
        ("unknown=1", "unknown parameter"),
    ] {
        let values = annotations(&format!("@upgrade(protocol=\"fastnet\",{argument})"));
        assert!(
            parse_websocket_config(&values, UpgradeMode::Raw)
                .unwrap_err()
                .contains(fragment)
        );
    }
    assert!(
        parse_websocket_config(
            &annotations("@upgrade(protocol=\"fastnet\")"),
            UpgradeMode::Raw
        )
        .unwrap()
        .is_none()
    );
    assert!(
        parse_websocket_config(&[], UpgradeMode::WebSocket)
            .unwrap()
            .is_none()
    );
    assert!(
        parse_upgrade_protocol(&annotations("@upgrade(codec=\"json\")"))
            .unwrap_err()
            .contains("requires a protocol")
    );
}
