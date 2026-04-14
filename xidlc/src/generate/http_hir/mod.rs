mod http_hir_attr;
mod http_hir_codegen;
mod http_hir_model;
mod http_hir_project;
mod http_hir_project_params;
mod http_hir_route;
mod http_hir_validate;

pub mod semantics;

pub(crate) use http_hir_codegen::HttpHirCodegen;
pub use http_hir_model::*;
pub use http_hir_project::project;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use xidl_parser::hir;

    fn parse(source: &str) -> hir::Specification {
        let typed = xidl_parser::parser::parser_text(source).expect("parse idl");
        hir::Specification::from_typed_ast_with_properties_and_path(
            typed,
            HashMap::new(),
            std::path::Path::new("input.idl"),
        )
        .expect("project hir")
    }

    #[test]
    fn projects_normalized_http_operations() {
        let spec = parse(
            r#"
            module api {
              interface CityApi {
                @get(path="/cities/{id}{?region}")
                string get_city(@path("id") in string id, @query("region") in string region, @header("X-Trace-Id") in string trace, out string etag);
              };
            };
            "#,
        );
        let doc = project(&spec).expect("http hir");
        let op = &doc.interfaces[0].operations[0];
        assert_eq!(op.method, HttpMethod::Get);
        assert_eq!(op.routes[0].path, "/cities/{id}");
        assert_eq!(op.routes[0].path_params, vec!["id".to_string()]);
        assert_eq!(op.routes[0].query_params, vec!["region".to_string()]);
        assert_eq!(op.request_path_params[0].wire_name, "id");
        assert_eq!(op.request_query_params[0].wire_name, "region");
        assert_eq!(op.request_header_params[0].wire_name, "X-Trace-Id");
        assert_eq!(op.response_body_params[0].name, "etag");
    }

    #[test]
    fn projects_attribute_operations_and_document_metadata() {
        let spec = parse(
            r#"
            #pragma xidlc package = "petstore"
            #pragma xidlc service "https://example.com" "prod"
            interface DeviceApi {
              @server_stream readonly attribute string status;
            };
            "#,
        );
        let doc = project(&spec).expect("http hir");
        assert_eq!(doc.document.package.as_deref(), Some("petstore"));
        assert_eq!(doc.document.servers[0].base_url, "https://example.com");
        let ops = &doc.interfaces[0].operations;
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].source, HttpOperationSource::AttributeGet);
        assert_eq!(ops[1].source, HttpOperationSource::AttributeWatch);
        assert_eq!(ops[1].stream.kind, Some(semantics::HttpStreamKind::Server));
    }
}
