use crate::hir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::annotations::{annotation_name, annotation_params, normalize_annotation_params};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpgradeMode {
    WebSocket,
    Raw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebSocketCodec {
    Json,
    Msgpack,
    Bytes,
}

/// WebSocket-mode configuration parsed from `@upgrade(protocol = "websocket", ...)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub codec: WebSocketCodec,
    pub subprotocol: Option<String>,
    /// Active ping interval in milliseconds. `None` or `0` means no active ping.
    pub heartbeat_ms: Option<u64>,
    pub max_message_bytes: Option<u64>,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            codec: WebSocketCodec::Json,
            subprotocol: None,
            heartbeat_ms: None,
            max_message_bytes: None,
        }
    }
}

pub fn classify_upgrade_protocol(protocol: &str) -> Result<UpgradeMode, String> {
    let trimmed = protocol.trim();
    if trimmed.is_empty() {
        return Err("@upgrade protocol parameter cannot be empty".to_string());
    }
    let lower = trimmed.to_ascii_lowercase();
    match lower.as_str() {
        "websocket" => Ok(UpgradeMode::WebSocket),
        "ws" | "wss" => Err(
            "@upgrade protocol cannot be \"ws\" or \"wss\"; use \"websocket\" instead".to_string(),
        ),
        _ => Ok(UpgradeMode::Raw),
    }
}

fn find_upgrade_params(annotations: &[hir::Annotation]) -> Option<HashMap<String, String>> {
    let annotation = annotations.iter().find(|annotation| {
        annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case("upgrade"))
            .unwrap_or(false)
    })?;
    annotation_params(annotation).map(normalize_annotation_params)
}

pub fn parse_upgrade_protocol(annotations: &[hir::Annotation]) -> Result<Option<String>, String> {
    let Some(params) = find_upgrade_params(annotations) else {
        return Ok(None);
    };
    let Some(proto) = params.get("protocol").cloned() else {
        return Err(
            "@upgrade annotation requires a protocol parameter (e.g. @upgrade(protocol=\"xidl-raw\"))"
                .to_string(),
        );
    };
    Ok(Some(proto))
}

pub fn parse_websocket_config(
    annotations: &[hir::Annotation],
    mode: UpgradeMode,
) -> Result<Option<WebSocketConfig>, String> {
    let Some(params) = find_upgrade_params(annotations) else {
        return Ok(None);
    };
    if mode != UpgradeMode::WebSocket {
        for key in ["codec", "subprotocol", "heartbeat", "max_message"] {
            if params.contains_key(key) {
                return Err(format!(
                    "@upgrade parameter {key} is only valid with protocol = \"websocket\""
                ));
            }
        }
        for key in params.keys() {
            if key != "protocol" {
                return Err(format!("@upgrade unknown parameter {key}"));
            }
        }
        return Ok(None);
    }

    let mut config = WebSocketConfig::default();
    if let Some(codec) = params.get("codec") {
        config.codec = match codec.to_ascii_lowercase().as_str() {
            "json" => WebSocketCodec::Json,
            "msgpack" => WebSocketCodec::Msgpack,
            "bytes" => WebSocketCodec::Bytes,
            other => {
                return Err(format!(
                    "unsupported @upgrade codec {other}, expected json, msgpack, or bytes"
                ));
            }
        };
    }
    if let Some(sub) = params.get("subprotocol") {
        validate_websocket_subprotocol(sub)?;
        config.subprotocol = Some(sub.clone());
    }
    if let Some(heartbeat) = params.get("heartbeat") {
        config.heartbeat_ms = Some(parse_duration_ms(heartbeat)?);
    }
    if let Some(max_message) = params.get("max_message") {
        let value = max_message.parse::<u64>().map_err(|_| {
            format!("@upgrade max_message must be a positive integer, got {max_message}")
        })?;
        if value < 1 {
            return Err("@upgrade max_message must be >= 1".to_string());
        }
        config.max_message_bytes = Some(value);
    }
    for key in params.keys() {
        match key.as_str() {
            "protocol" | "codec" | "subprotocol" | "heartbeat" | "max_message" => {}
            other => {
                return Err(format!("@upgrade unknown parameter {other}"));
            }
        }
    }
    Ok(Some(config))
}

pub fn validate_websocket_subprotocol(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("@upgrade subprotocol cannot be empty".to_string());
    }
    // RFC 6455 token: 1*( %x21 / %x23-2B / %x2D-3A / %x3C-5B / %x5D-7E )
    let ok = value.bytes().all(|b| {
        b == 0x21
            || (0x23..=0x2B).contains(&b)
            || (0x2D..=0x3A).contains(&b)
            || (0x3C..=0x5B).contains(&b)
            || (0x5D..=0x7E).contains(&b)
    });
    if !ok {
        return Err(format!(
            "@upgrade subprotocol must be an RFC 6455 token, got {value}"
        ));
    }
    Ok(())
}

/// Parses duration strings such as `"500ms"`, `"20s"`, `"1m"`, or `"0"`.
pub fn parse_duration_ms(value: &str) -> Result<u64, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("@upgrade heartbeat cannot be empty".to_string());
    }
    let parse_num = |num: &str| -> Result<u64, String> {
        num.parse::<u64>()
            .map_err(|_| format!("invalid @upgrade heartbeat duration {value}"))
    };
    if let Some(num) = trimmed.strip_suffix("ms") {
        return parse_num(num);
    }
    if let Some(num) = trimmed.strip_suffix('s') {
        return Ok(parse_num(num)?.saturating_mul(1000));
    }
    if let Some(num) = trimmed.strip_suffix('m') {
        return Ok(parse_num(num)?.saturating_mul(60_000));
    }
    // bare integer is treated as milliseconds
    parse_num(trimmed)
}

#[cfg(test)]
mod tests {
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
}
