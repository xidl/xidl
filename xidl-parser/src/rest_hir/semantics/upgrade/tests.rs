use super::*;

#[test]
fn classify_protocol_modes() {
    assert_eq!(
        classify_upgrade_protocol("websocket").unwrap(),
        UpgradeMode::WebSocket
    );
    assert_eq!(
        classify_upgrade_protocol("WebSocket").unwrap(),
        UpgradeMode::WebSocket
    );
    assert_eq!(
        classify_upgrade_protocol("fastnet").unwrap(),
        UpgradeMode::Raw
    );
    assert!(classify_upgrade_protocol("ws").is_err());
    assert!(classify_upgrade_protocol("wss").is_err());
    assert!(classify_upgrade_protocol("").is_err());
}

#[test]
fn parse_duration_variants() {
    assert_eq!(parse_duration_ms("500ms").unwrap(), 500);
    assert_eq!(parse_duration_ms("20s").unwrap(), 20_000);
    assert_eq!(parse_duration_ms("1m").unwrap(), 60_000);
    assert_eq!(parse_duration_ms("0").unwrap(), 0);
    assert!(parse_duration_ms("abc").is_err());
}

#[test]
fn subprotocol_token_rules() {
    assert!(validate_websocket_subprotocol("fastnet.v1").is_ok());
    assert!(validate_websocket_subprotocol("").is_err());
    assert!(validate_websocket_subprotocol("bad token").is_err());
}
