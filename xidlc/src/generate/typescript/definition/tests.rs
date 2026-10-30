use std::collections::HashMap;

use crate::generate::typescript::{TsMode, TypescriptRenderer};
use xidl_parser::hir;

use super::render_typescript;

fn parse_spec(source: &str) -> hir::Specification {
    let typed = xidl_parser::parser::parser_text(source).expect("parse typed ast");
    hir::Specification::from(typed)
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
            @path @rename("bucket") string bucket,
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

#[test]
fn rooted_interface_envelopes_import_their_file_namespace_only_when_rendered() {
    let typed = xidl_parser::parser::parser_text(
        r#"
        struct Payload { string id; };
        interface Api {
            ::Payload read(in ::Payload value, out ::Payload previous);
        };
        "#,
    )
    .expect("rooted interface IDL");
    let spec = hir::Specification::from(typed);
    let mut renderer = TypescriptRenderer::new().expect("renderer");
    renderer.extend(&HashMap::new());
    for mode in [TsMode::All, TsMode::InterfaceOnly] {
        let output = render_typescript(&spec, "scope", &renderer, mode).expect("rooted interface");
        assert!(
            output
                .types
                .contains("import * as $xidl_root from \"./scope\";")
        );
        assert!(output.types.contains("$xidl_root.Payload"));
        assert!(
            output
                .zod
                .contains("import * as $xidl_root from \"./scope.zod\";")
        );
        assert!(output.zod.contains("$xidl_root.PayloadSchema"));
    }
    let output =
        render_typescript(&spec, "scope", &renderer, TsMode::TypesOnly).expect("models only");
    assert!(!output.types.contains("$xidl_root"));
    assert!(!output.zod.contains("$xidl_root"));
}
