use super::parse;
use crate::rest_hir::{
    project,
    semantics::{HttpStreamCodec, HttpStreamKind},
};

#[test]
fn structured_streams_keep_defaults_and_resolve_declared_exceptions() {
    let spec = parse(
        r#"
        module billing {
            @http(409) exception Conflict { string reason; };
            interface Export {
                @server_stream string rows() raises(Conflict);
                @client_stream int32 upload(in int32 row) raises(::billing::Conflict);
            };
        };
    "#,
    );
    let document = project(&spec).unwrap();
    let operations = &document.interfaces[0].operations;
    assert_eq!(operations[0].meta.stream.kind, Some(HttpStreamKind::Server));
    assert_eq!(operations[0].meta.stream.codec, HttpStreamCodec::Sse);
    assert_eq!(operations[1].meta.stream.codec, HttpStreamCodec::Ndjson);
    assert!(
        operations
            .iter()
            .all(|op| op.meta.raises[0].module_path == ["billing"])
    );
}

#[test]
fn stream_exceptions_require_an_in_band_body_and_leave_unary_bindings_intact() {
    for annotation in ["@header", "@cookie"] {
        let exception = format!("@http(409) exception Conflict {{ {annotation} string detail; }};");
        let unary = parse(&format!(
            "{exception} interface Api {{ string read() raises(Conflict); }};"
        ));
        project(&unary).unwrap();
        for stream in ["@server_stream", "@client_stream"] {
            let spec = parse(&format!(
                "{exception} interface Api {{ {stream} string read() raises(Conflict); }};"
            ));
            let error = project(&spec).unwrap_err().to_string();
            assert!(
                error.contains("Conflict") && error.contains("header or cookie"),
                "{error}"
            );
        }
    }
}

#[test]
fn raw_download_and_websocket_raises_stay_explicitly_unsupported() {
    for method in [
        "@server_stream @get sequence<octet> read() raises(Conflict);",
        "@bidi_stream string read(in string item) raises(Conflict);",
    ] {
        let spec = parse(&format!(
            "@http(409) exception Conflict {{}}; interface Api {{ {method} }};"
        ));
        let error = project(&spec).unwrap_err().to_string();
        assert!(error.contains("read"), "{error}");
        assert!(
            error.contains("raw byte") || error.contains("WebSocket"),
            "{error}"
        );
    }
    let raw_upload = parse(
        "@http(409) exception Conflict {}; interface Api { @client_stream string upload(in sequence<octet> chunk) raises(Conflict); };",
    );
    project(&raw_upload).unwrap();
}
