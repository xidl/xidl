use std::collections::HashMap;
use std::path::Path;

use crate::hir;
use crate::rest_hir::{HttpOperationPublication, project};

fn lower(source: &str) -> crate::error::ParserResult<hir::Specification> {
    let ast = crate::parser::parser_text(source)?;
    hir::Specification::from_typed_ast_with_properties_and_path(
        ast,
        HashMap::new(),
        Path::new("publication.idl"),
    )
}

#[test]
fn marker_only_changes_publication_of_normal_http_operations() {
    let source = r#"
        @http(409) exception Rejected { string reason; };
        @http_bearer interface Receiver {
            @get(path="/events") boolean ready();
            @webhooks @put(path="/events")
            string receive(@header(name="X-ID") string id, @body string event)
                raises(Rejected);
        };
    "#;
    let marked = project(&lower(source).unwrap()).unwrap();
    let plain = project(&lower(&source.replace("@webhooks", "")).unwrap()).unwrap();
    assert_eq!(
        marked.interfaces[0].operations[0].meta.publication,
        HttpOperationPublication::Path
    );
    let operation = &marked.interfaces[0].operations[1];
    assert_eq!(
        operation.meta.publication,
        HttpOperationPublication::Webhook
    );
    let mut actual = serde_json::to_value(operation).unwrap();
    actual["meta"]
        .as_object_mut()
        .unwrap()
        .remove("publication");
    assert_eq!(
        actual,
        serde_json::to_value(&plain.interfaces[0].operations[1]).unwrap()
    );
}

#[test]
fn interface_marker_covers_attributes_without_mutating_other_interfaces() {
    let source = r#"
        @webhooks interface Receiver {
            @webhooks void receive(@body string event);
            attribute string state;
        };
        interface Plain { void receive(@body string event); };
    "#;
    let document = project(&lower(source).unwrap()).unwrap();
    assert_eq!(document.interfaces[0].operations.len(), 3);
    assert!(
        document.interfaces[0]
            .operations
            .iter()
            .all(|operation| operation.meta.publication == HttpOperationPublication::Webhook)
    );
    assert_eq!(
        document.interfaces[1].operations[0].meta.publication,
        HttpOperationPublication::Path
    );
}

#[test]
fn qualified_custom_annotation_does_not_alias_builtin_marker() {
    let source = r#"
        @vendor::webhooks(signature="custom")
        interface Receiver { void receive(@body string event); };
    "#;
    let document = project(&lower(source).unwrap()).unwrap();
    assert_eq!(
        document.interfaces[0].operations[0].meta.publication,
        HttpOperationPublication::Path
    );
}

#[test]
fn all_source_targets_validate_the_marker_before_version_projection() {
    for source in [
        "@webhooks module Invalid {};",
        "@webhooks struct Invalid { string value; };",
        "struct Invalid { @webhooks string value; };",
        "enum Invalid { @webhooks value };",
        "interface Invalid { void call(@webhooks string value); };",
        "interface Invalid { @webhooks attribute string value; };",
        "@version enum V { v1,v2 }; @removed(V::v2) @webhooks struct Invalid {};",
    ] {
        let error = lower(source).unwrap_err().to_string();
        assert!(
            error.contains("@webhooks is not allowed"),
            "{source}: {error}"
        );
        assert!(error.contains("bytes"), "{error}");
    }
    for arguments in [
        "signature=\"standard\"",
        "envelope=Meta",
        "name=\"event\"",
        "true",
    ] {
        let source = format!("@webhooks({arguments}) interface Invalid {{ void call(); }};");
        let error = lower(&source).unwrap_err().to_string();
        assert!(error.contains("takes no arguments"), "{error}");
        assert!(error.contains("application"), "{error}");
    }
}

#[test]
fn duplicate_and_case_insensitive_markers_remain_one_publication() {
    let document = project(
        &lower("@WEBHOOKS interface Receiver { @webhooks @webhooks void receive(); };").unwrap(),
    )
    .unwrap();
    assert_eq!(document.interfaces[0].operations.len(), 1);
    assert_eq!(
        document.interfaces[0].operations[0].meta.publication,
        HttpOperationPublication::Webhook
    );
}
