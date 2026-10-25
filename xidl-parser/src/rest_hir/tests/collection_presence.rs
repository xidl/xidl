use super::parse;
use crate::rest_hir::{HttpParamKind, project};

#[test]
fn collection_wire_presence_preserves_required_logical_arguments() {
    let spec = parse(
        r#"
        interface Inputs {
            @get(path="/inputs/{id}")
            void read(@path string id, @query long count,
                      @query sequence<string> values,
                      @header sequence<string> tags,
                      @cookie sequence<string> names,
                      @query @optional sequence<string> labels);
        };
        "#,
    );
    let document = project(&spec).expect("collection HTTP projection");
    let operation = &document.interfaces[0].operations[0];
    let request = &operation.http.request;
    assert!(request.path[0].requires_presence(HttpParamKind::Path));
    assert!(request.query[0].requires_presence(HttpParamKind::Query));
    for (location, binding) in [
        (HttpParamKind::Query, &request.query[1]),
        (HttpParamKind::Header, &request.header[0]),
        (HttpParamKind::Cookie, &request.cookie[0]),
    ] {
        assert!(
            !binding.optional,
            "{} remains a required argument",
            binding.source_param
        );
        assert!(!binding.requires_presence(location));
        assert!(binding.requires_presence(HttpParamKind::Path));
    }
    assert!(request.query[2].optional);
    assert!(!request.query[2].requires_presence(HttpParamKind::Query));
    let parameters = &operation.signature.params;
    assert!(
        parameters[..5]
            .iter()
            .all(|parameter| !parameter.is_optional)
    );
    assert!(parameters[5].is_optional);
}
