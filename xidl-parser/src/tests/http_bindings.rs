use crate::{hir, parser::parser_text, rest_hir};

fn lower(source: &str) -> hir::Specification {
    hir::Specification::from_typed_ast_with_properties_and_path(
        parser_text(source).expect("IDL syntax"),
        [("expand_interface".into(), false.into())].into(),
        "bindings.idl",
    )
    .expect("HIR")
}

fn project(source: &str) -> rest_hir::RestHirDocument {
    rest_hir::project(&lower(source)).expect("HTTP bindings")
}

#[test]
fn conditional_update_shares_wire_names_across_input_output_and_typed_errors() {
    let source = r#"
        @http(429) @rename_all("camelCase") exception Limited {
            @header("Retry-After") long retry_after;
            @cookie(name="session") string session_id;
            string error_message;
        };
        interface Messages {
            @patch(path="/messages/{id}")
            string edit(@path(path="id") string message_id,
                @header("Idempotency-Key") string key,
                @query(name="expand") string fields,
                @cookie("session") string session_id,
                @header("ignored") @rename("If-Match") string revision,
                @header(name="ETag") out string etag) raises(Limited);
        };
    "#;
    let document = project(source);
    let operation = &document.interfaces[0].operations[0];
    assert_eq!(operation.http.request.path[0].wire_name, "id");
    assert_eq!(
        operation.http.request.header[0].wire_name,
        "Idempotency-Key"
    );
    assert_eq!(operation.http.request.header[1].wire_name, "If-Match");
    assert_eq!(operation.http.request.query[0].wire_name, "expand");
    assert_eq!(operation.http.request.cookie[0].wire_name, "session");
    assert_eq!(operation.http.response.header[0].wire_name, "ETag");
    assert!(operation.signature.params[1].annotations.contains(
        &rest_hir::HttpSignatureParamAnnotation::Header {
            name: "Idempotency-Key".into()
        }
    ));
    let error = &document.document.exceptions[0];
    assert_eq!(error.headers[0].wire_name, "Retry-After");
    assert_eq!(error.cookies[0].wire_name, "session");
    assert_eq!(error.body[0].wire_name, "errorMessage");
}

#[test]
fn repeated_source_annotations_are_order_independent_and_conflicts_fail() {
    for annotations in [
        r#"@header @header("X-Trace")"#,
        r#"@header("X-Trace") @header"#,
        r#"@header("X-Trace") @header(name="X-Trace")"#,
        r#"@header("X-Trace") @header(name="x-trace")"#,
    ] {
        let document = project(&format!(
            "interface I {{ @get(path=\"/x\") void get({annotations} string trace); }};"
        ));
        assert_eq!(
            document.interfaces[0].operations[0].http.request.header[0].wire_name,
            "X-Trace"
        );
    }
    for annotations in [
        r#"@header("X") @header("Y")"#,
        r#"@header("X") @cookie("X")"#,
        r#"@header(name="X", value="Y")"#,
        r#"@header("X", "Y")"#,
        r#"@header(nmae="X")"#,
        r#"@header(123)"#,
        r#"@header("invalid header")"#,
        r#"@header(":status")"#,
        r#"@cookie("invalid;name")"#,
        r#"@cookie("invalid,name")"#,
        r#"@cookie("invalid\\name")"#,
        r#"@body(name="unsupported")"#,
    ] {
        let source = format!("interface I {{ void send({annotations} string value); }};");
        assert!(rest_hir::project(&lower(&source)).is_err(), "{annotations}");
    }
    let source = r#"@http(429) exception Limited { @header @cookie string value; };"#;
    assert!(rest_hir::project(&lower(source)).is_err());
    for source in ["path", "query"] {
        let source = format!("@http(429) exception Limited {{ @{source} string value; }};");
        assert!(rest_hir::project(&lower(&source)).is_err());
    }
}

#[test]
fn automatic_paths_and_input_uniqueness_use_effective_names_without_affecting_outputs() {
    let document = project(r#"interface I { void read(@path("wire_id") string id); };"#);
    assert_eq!(
        document.interfaces[0].operations[0].meta.routes[0].path,
        "/read/{wire_id}"
    );
    let document = project(
        r#"interface I { @get(path="/x/{wire}{?filter}")
            void read(@path @rename("wire") string id,
                @query @rename("filter") string query); };"#,
    );
    let request = &document.interfaces[0].operations[0].http.request;
    assert_eq!(request.path[0].wire_name, "wire");
    assert_eq!(request.query[0].wire_name, "filter");
    for parameters in [
        r#"@header("X-Tag") string one, @header("x-tag") string two"#,
        r#"@query("tag") string one, @query(name="tag") string two"#,
        r#"@cookie("tag") string one, @cookie(name="tag") string two"#,
        r#"@body("tag") string one, @body("tag") string two"#,
    ] {
        assert!(
            rest_hir::project(&lower(&format!(
                "interface I {{ void send({parameters}); }};"
            )))
            .is_err(),
            "{parameters}"
        );
    }
    let document = project(
        r#"interface I {
        @get(path="/x") void read(@query("tag") string one, @query("Tag") string two,
            @header("tag") string three, @cookie("tag") string four,
            @header("X-Tag") out sequence<string> z,
            @header("X-Tag") out sequence<string> a);
    };"#,
    );
    assert_eq!(
        document.interfaces[0].operations[0]
            .http
            .response
            .header
            .len(),
        2
    );
}

#[test]
fn map_parameter_identities_follow_projected_bindings_instead_of_reparsing_names() {
    let source = r#"@version enum V { v1, v2 };
        interface I { @post(path="/x") void send(
            @header("Idempotency-Key") string value,
            @query(name="request_id") string value,
            @header("ETag") out string value,
            @added(V::v2) @cookie("audit") string audit);
        };
    "#;
    let specification = lower(source);
    let record = crate::versioning::VersioningRecord::analyze(&specification)
        .unwrap()
        .unwrap();
    let map = crate::versioning::map::from_spec(&specification, &record, "bindings.idl").unwrap();
    let from = map.steps[0].operations["I.send"].from.as_ref().unwrap();
    let parameters = &from["parameters"];
    assert_eq!(
        parameters["header:Idempotency-Key"]["wire_name"],
        "Idempotency-Key"
    );
    assert_ne!(
        parameters["header:Idempotency-Key"]["declaration_id"],
        parameters["query:request_id"]["declaration_id"]
    );
    assert_eq!(
        from["response_parameters"]["header:ETag"][0]["wire_name"],
        "ETag"
    );
}

#[test]
fn raw_source_arguments_validate_and_read_the_same_comment_free_shape() {
    use crate::rest_hir::semantics::SourceBinding;

    for raw in [
        r#""X-Trace""#,
        r#"name /* key */ = "X-Trace" /* value */"#,
        r#"/* before */ value="X-Trace""#,
    ] {
        let annotations = [hir::Annotation::Builtin {
            name: "header".into(),
            params: Some(hir::AnnotationParams::Raw(raw.into())),
        }];
        let binding = SourceBinding::read(&annotations, "trace").unwrap().unwrap();
        assert_eq!(binding.wire_name("trace", &annotations, &[]), "X-Trace");
    }
    for raw in [r#"name=="X-Trace""#, r#"name="X", name="Y""#, "name=123"] {
        let annotations = [hir::Annotation::Builtin {
            name: "query".into(),
            params: Some(hir::AnnotationParams::Raw(raw.into())),
        }];
        assert!(SourceBinding::read(&annotations, "trace").is_err(), "{raw}");
    }
    for raw in [r#""a=b""#, r#"name="a=b""#] {
        let annotations = [hir::Annotation::Builtin {
            name: "query".into(),
            params: Some(hir::AnnotationParams::Raw(raw.into())),
        }];
        let binding = SourceBinding::read(&annotations, "filter")
            .unwrap()
            .unwrap();
        assert_eq!(binding.wire_name("filter", &annotations, &[]), "a=b");
    }
}
