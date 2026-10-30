use crate::{hir, parser::parser_text, rest_hir::*};

fn project_contract(source: &str) -> Result<RestHirDocument, String> {
    let spec = hir::Specification::from_typed_ast_with_properties_and_path(
        parser_text(source).expect("contract syntax"),
        Default::default(),
        "workflow.idl",
    )
    .expect("contract HIR");
    project(&spec).map_err(|error| error.to_string())
}

#[test]
fn binary_upload_binding_distinguishes_the_entire_body_from_object_fields() {
    let document = project_contract(
        r#"
interface Uploads {
    @post(path="/raw") void raw(@body sequence<octet> data);
    @post(path="/text") void text(@body string data);
    @post(path="/record") void record(sequence<octet> data, string checksum);
    @get(path="/status") string status();
};
"#,
    )
    .expect("upload HTTP contract");
    let actual = document.interfaces[0]
        .operations
        .iter()
        .map(|operation| {
            (
                operation.meta.name.as_str(),
                operation.http.request.body.shape.byte_parameter(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        [
            ("raw", Some("data")),
            ("text", None),
            ("record", None),
            ("status", None),
        ]
    );
}

#[test]
fn conditional_file_update_preserves_each_wire_binding_and_typed_failure() {
    let document = project_contract(
        r#"
module files {
    struct Entry { string name; };
    @http(status=412) exception Precondition {
        @header @optional sequence<string> reasons;
        @cookie string session;
        @body @rename("message") string why;
    };
    interface Store {
        @patch(path="/files/{id}{?revision}")
        Entry update(@path string id, @query @optional uint32 revision,
            @header @rename("If-Match") string etag, @cookie string session,
            @body @flatten Entry entry, @header @rename("ETag") out string next_etag,
            @cookie out string rotated) raises(Precondition);
    };
};
"#,
    )
    .unwrap();
    let operation = &document.interfaces[0].operations[0];
    assert_eq!(operation.meta.method, HttpMethod::Patch);
    assert_eq!(operation.meta.routes[0].path, "/files/{id}");
    let request = &operation.http.request;
    assert_eq!(request.path[0].source_param, "id");
    assert_eq!(request.query[0].wire_name, "revision");
    assert!(request.query[0].optional);
    assert_eq!(request.header[0].wire_name, "If-Match");
    assert_eq!(request.cookie[0].wire_name, "session");
    assert!(
        matches!(&request.body.shape, HttpRequestBodyShape::SingleValue { source_param, flatten: true, .. } if source_param == "entry")
    );
    assert_eq!(
        request.body.content_type.as_deref(),
        Some("application/json")
    );
    let entry = operation
        .signature
        .params
        .iter()
        .find(|p| p.name == "entry")
        .unwrap();
    assert!(
        entry
            .annotations
            .contains(&HttpSignatureParamAnnotation::Flatten)
    );
    assert!(
        entry
            .annotations
            .contains(&HttpSignatureParamAnnotation::Body)
    );
    let response = &operation.http.response;
    assert_eq!(response.status, "200");
    assert_eq!(response.header[0].wire_name, "ETag");
    assert_eq!(response.cookie[0].wire_name, "rotated");
    assert!(matches!(
        response.body.shape,
        HttpResponseBodyShape::ReturnOnly { .. }
    ));
    let error = &document.document.exceptions[0];
    assert_eq!(error.status, 412);
    assert!(error.headers[0].optional && error.headers[0].is_multi);
    assert_eq!(error.cookies[0].wire_name, "session");
    assert_eq!(error.body[0].wire_name, "message");
    assert_eq!(operation.meta.raises[0].module_path, ["files"]);
}

#[test]
fn blob_and_event_workflows_select_their_framing_and_default_media_types() {
    let document = project_contract(
        r#"
interface Blobs {
    @post(path="/blob") void put(@body sequence<octet> payload);
    @client_stream void upload(@body sequence<octet> chunks);
    @server_stream sequence<octet> download();
    @server_stream void download_out(out sequence<octet> chunks);
    @server_stream @stream_codec("ndjson") string events();
};
"#,
    )
    .unwrap();
    let operations = &document.interfaces[0].operations;
    let put = &operations[0].http;
    assert!(matches!(
        put.request.body.shape,
        HttpRequestBodyShape::SingleValue { .. }
    ));
    assert_eq!(
        put.request.body.content_type.as_deref(),
        Some("application/octet-stream")
    );
    assert_eq!(put.response.status, "204");
    let upload = &operations[1].http.request.body;
    assert!(
        matches!(&upload.shape, HttpRequestBodyShape::Stream { source_param, codec: HttpStreamPayloadCodec::Bytes, .. } if source_param == "chunks")
    );
    for operation in &operations[2..4] {
        assert_eq!(operation.http.response.status, "200");
        assert_eq!(
            operation.http.response.body.content_type.as_deref(),
            Some("application/octet-stream")
        );
        assert!(matches!(
            operation.http.response.body.shape,
            HttpResponseBodyShape::Stream {
                codec: HttpStreamPayloadCodec::Bytes,
                ..
            }
        ));
    }
    assert!(matches!(
        operations[2].http.response.body.shape,
        HttpResponseBodyShape::Stream {
            item_source: HttpOutputSource::ReturnValue,
            ..
        }
    ));
    assert!(
        matches!(&operations[3].http.response.body.shape, HttpResponseBodyShape::Stream { item_source: HttpOutputSource::Param { name }, .. } if name == "chunks")
    );
    assert_eq!(
        operations[4].http.response.body.content_type.as_deref(),
        Some("application/x-ndjson")
    );
    assert!(matches!(
        operations[4].http.response.body.shape,
        HttpResponseBodyShape::Stream {
            codec: HttpStreamPayloadCodec::Ndjson,
            ..
        }
    ));
}

#[test]
fn command_routes_keep_default_object_bodies_and_head_metadata() {
    let document = project_contract(
        r#"
interface Commands {
    @post(path="/items") void create(@body string name, @body @optional string note);
    @put(path="/items") void replace(@body string value);
    @head(path="/items") void probe(@header out string etag);
    @options(path="/items") void capabilities();
    @oneway void notify(@body string message);
};
"#,
    )
    .unwrap();
    let operations = &document.interfaces[0].operations;
    let request = &operations[0].http.request.body;
    assert_eq!(request.content_type.as_deref(), Some("application/json"));
    let HttpRequestBodyShape::Object { fields } = &request.shape else {
        panic!("object body")
    };
    assert_eq!(
        fields
            .iter()
            .map(|p| (p.field_name.as_str(), p.optional))
            .collect::<Vec<_>>(),
        [("name", false), ("note", true)]
    );
    assert_eq!(
        operations.iter().map(|p| p.meta.method).collect::<Vec<_>>(),
        [
            HttpMethod::Post,
            HttpMethod::Put,
            HttpMethod::Head,
            HttpMethod::Options,
            HttpMethod::Post
        ]
    );
    assert!(matches!(
        operations[2].http.response.body.shape,
        HttpResponseBodyShape::Empty
    ));
    assert_eq!(operations[2].http.response.header[0].wire_name, "etag");
    assert_eq!(operations[4].meta.routes[0].path, "/notify");
}

#[test]
fn failures_report_the_invalid_exception_or_route_contract() {
    for (source, expected) in [
        (
            "@http(422) exception Invalid { string names[2]; };",
            vec!["Invalid", "array declarator"],
        ),
        (
            "exception MissingStatus { string message; };",
            vec!["MissingStatus", "@http"],
        ),
        (
            "@http(other=412) exception Invalid { string message; };",
            vec!["Invalid", "status"],
        ),
        (
            "@http(\"not-a-status\") exception Invalid { string message; };",
            vec!["Invalid", "not-a-status"],
        ),
        (
            "interface Files { string read() raises(Missing); };",
            vec!["read", "Missing"],
        ),
        (
            "@http(409) exception A { string message; }; @http(409) exception B { string message; }; interface Files { void read() raises(A, B); };",
            vec!["read", "409"],
        ),
        (
            "interface Files { @get(path=\"/files{?cursor}/tail\") void read(); };",
            vec!["query template", "terminate"],
        ),
        (
            "interface Files { @get(path=\"/files{?cursor,}\") void read(); };",
            vec!["empty", "query"],
        ),
        (
            "interface Files { @get(path=\"/files/{}\") void read(); };",
            vec!["empty", "path"],
        ),
        (
            "interface Files { @get(path=\"/files\") void read(@path string id); };",
            vec!["id", "route"],
        ),
    ] {
        let error = project_contract(source).unwrap_err();
        for fragment in expected {
            assert!(
                error.contains(fragment),
                "{source}: {error} lacks {fragment}"
            );
        }
    }
}
