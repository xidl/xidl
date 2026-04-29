use std::collections::HashMap;

use crate::generate::typescript::{TsMode, TypescriptRenderer};
use xidl_parser::hir;

use super::render_typescript;

fn parse_spec(source: &str) -> hir::Specification {
    let typed = xidl_parser::parser::parser_text(source).expect("parse typed ast");
    hir::Specification::from_typed_ast_with_properties(typed, HashMap::new())
}

#[test]
fn rejects_bidi_stream_methods() {
    let spec = parse_spec(
        r#"
        interface StreamApi {
          @bidi_stream
          void chat(
            in string room,
            in string text,
            out string from,
            out string reply
          );
        };
        "#,
    );
    let renderer = TypescriptRenderer::new().expect("renderer");
    let err = render_typescript(&spec, "stream_api", &renderer, TsMode::InterfaceOnly)
        .err()
        .expect("bidi stream should be rejected");
    assert!(err.to_string().contains("@bidi_stream"));
}

#[test]
fn rejects_client_stream_with_non_body_inputs() {
    let spec = parse_spec(
        r#"
        interface StreamApi {
          @client_stream
          @stream_codec("ndjson")
          @path("/upload/{bucket}")
          string upload(
            @path("bucket") string bucket,
            in sequence<octet> chunk
          );
        };
        "#,
    );
    let renderer = TypescriptRenderer::new().expect("renderer");
    let err = render_typescript(&spec, "stream_api", &renderer, TsMode::InterfaceOnly)
        .err()
        .expect("non-body client stream should be rejected");
    assert!(err.to_string().contains("body parameters only"));
}
