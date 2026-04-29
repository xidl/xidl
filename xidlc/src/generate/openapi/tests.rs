#[cfg(test)]
mod tests {
    use crate::generate::http_hir;
    use crate::generate::openapi::context::{patch_openapi_stream_content, render_openapi};
    use serde_json::Value;
    use std::collections::HashMap;
    use std::panic::{self, AssertUnwindSafe};
    use xidl_parser::hir;

    fn parse_spec(source: &str) -> hir::Specification {
        let typed = xidl_parser::parser::parser_text(source).expect("parse typed ast");
        hir::Specification::from_typed_ast_with_properties(typed, HashMap::new())
    }

    fn render_openapi_json_from_spec(
        spec: &hir::Specification,
    ) -> Result<Value, serde_json::Error> {
        let http_hir = http_hir::project(spec).expect("project http hir");
        let ctx = render_openapi(spec, &http_hir);
        let version = if ctx.stream_patches.is_empty() {
            "3.1.0"
        } else {
            "3.2.0"
        };
        let mut value = serde_json::to_value(ctx.document)?;
        if let Some(openapi) = value.get_mut("openapi") {
            *openapi = Value::String(version.to_string());
        }
        for patch in ctx.stream_patches {
            patch_openapi_stream_content(&mut value, &patch);
        }
        Ok(value)
    }

    fn doc_annotation(text: &str) -> hir::Annotation {
        hir::Annotation::Builtin {
            name: "doc".to_string(),
            params: Some(hir::AnnotationParams::Raw(format!("\"{}\"", text))),
        }
    }

    #[test]
    fn schema_for_struct_applies_doc_to_fields() {
        use crate::generate::openapi::schema::schema_for_struct;
        use crate::openapi::RefOr;
        use crate::openapi::schema::Schema;

        let member = hir::Member {
            annotations: vec![doc_annotation("field doc")],
            ty: hir::TypeSpec::IntegerType(hir::IntegerType::I32),
            ident: vec![hir::Declarator::SimpleDeclarator(hir::SimpleDeclarator(
                "value".to_string(),
            ))],
            default: None,
            field_id: None,
        };
        let schema = schema_for_struct(&[member]);
        let RefOr::T(Schema::Object(object)) = schema else {
            panic!("expected object schema");
        };
        let Some(prop) = object.properties.get("value") else {
            panic!("missing value property");
        };
        let RefOr::T(Schema::Object(prop_obj)) = prop else {
            panic!("expected object property schema");
        };
        assert_eq!(prop_obj.description.as_deref(), Some("field doc"));
    }

    #[test]
    fn render_openapi_json_defaults_to_31_without_streams() {
        let spec = parse_spec(
            r#"
            interface HelloApi {
              string hello();
            };
            "#,
        );
        let doc = render_openapi_json_from_spec(&spec).expect("render openapi json");
        assert_eq!(
            doc.get("openapi"),
            Some(&Value::String("3.1.0".to_string()))
        );
    }

    #[test]
    fn render_openapi_json_uses_32_and_item_schema_for_streams() {
        let spec = parse_spec(
            r#"
            interface StreamApi {
              @server_stream
              @stream_codec("sse")
              string watch();

              @client_stream
              @stream_codec("ndjson")
              string upload(
                in string file_id,
                in sequence<octet> chunk
              );
            };
            "#,
        );
        let doc = render_openapi_json_from_spec(&spec).expect("render openapi json");
        assert_eq!(
            doc.get("openapi"),
            Some(&Value::String("3.2.0".to_string()))
        );

        let server_content =
            &doc["paths"]["/watch"]["get"]["responses"]["200"]["content"]["text/event-stream"];
        assert!(server_content.get("itemSchema").is_some());
        assert!(server_content.get("schema").is_none());

        let client_content =
            &doc["paths"]["/upload"]["post"]["requestBody"]["content"]["application/x-ndjson"];
        assert!(client_content.get("itemSchema").is_some());
        assert!(client_content.get("schema").is_none());
    }

    fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
        if let Some(message) = payload.downcast_ref::<&'static str>() {
            (*message).to_string()
        } else if let Some(message) = payload.downcast_ref::<String>() {
            message.clone()
        } else {
            "unknown panic payload".to_string()
        }
    }

    #[test]
    fn render_openapi_json_rejects_invalid_stream_codec() {
        let spec = parse_spec(
            r#"
            interface StreamApi {
              @server_stream
              @stream_codec("yaml")
              string watch();
            };
            "#,
        );
        let payload =
            panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json_from_spec(&spec)))
                .expect_err("invalid stream codec should panic");
        let message = panic_message(payload);
        assert!(message.contains("unsupported @stream_codec value"));
    }

    #[test]
    fn render_openapi_json_rejects_duplicate_route_bindings() {
        let spec = parse_spec(
            r#"
            interface HttpApi {
              @get(path = "/users/{id}")
              string get_user(@path("id") string id);

              @get(path = "/users/{id}")
              string fetch_user(@path("id") string id);
            };
            "#,
        );
        let payload =
            panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json_from_spec(&spec)))
                .expect_err("duplicate route binding should panic");
        let message = panic_message(payload);
        assert!(message.contains("duplicate HTTP route binding"));
    }
}
