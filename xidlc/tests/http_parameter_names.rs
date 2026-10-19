use std::{collections::HashMap, path::Path};

const LEGACY: &str = r#"
@http(429) exception Limited {
    @header @rename("Retry-After") long delay;
    @cookie @rename("session") string token;
};
interface Messages {
    @patch(path = "/messages/{message_id}{?expand}")
    string edit(@path @rename("message_id") string id,
                @query @rename("expand") @optional string fields,
                @header @rename("Idempotency-Key") @optional string key,
                @cookie @rename("session") @optional string token,
                @header @rename("ETag") out string revision) raises(Limited);
};
"#;

const NAMED: &str = r#"
@http(429) exception Limited {
    @header(name = "Retry-After") long delay;
    @cookie(name = "session") string token;
};
interface Messages {
    @patch(path = "/messages/{message_id}{?expand}")
    string edit(@path(name = "message_id") string id,
                @query(name = "expand") @optional string fields,
                @header(name = "Idempotency-Key") @optional string key,
                @cookie(name = "session") @optional string token,
                @header(name = "ETag") out string revision) raises(Limited);
};
"#;

const POSITIONAL: &str = r#"
@http(429) exception Limited {
    @header("Retry-After") long delay;
    @cookie("session") string token;
};
interface Messages {
    @patch(path = "/messages/{message_id}{?expand}")
    string edit(@path("message_id") string id,
                @query("expand") @optional string fields,
                @header("Idempotency-Key") @optional string key,
                @cookie("session") @optional string token,
                @header("ETag") out string revision) raises(Limited);
};
"#;

#[test]
fn shorthand_and_rename_generate_the_same_wire_names() {
    let mut documents = Vec::new();
    for source in [LEGACY, NAMED, POSITIONAL] {
        for target in ["rust-axum", "typescript-rest", "go-rest"] {
            xidlc::driver::Generator::new(target.to_string())
                .generate_from_idl(source, Path::new("api.idl"), HashMap::new())
                .unwrap_or_else(|error| panic!("{target}: {error}"));
        }
        let files = xidlc::driver::Generator::new("openapi".to_string())
            .generate_from_idl(source, Path::new("api.idl"), HashMap::new())
            .expect("OpenAPI generation");
        let document: serde_json::Value = serde_json::from_str(files[0].content()).unwrap();
        let parameters = &document["paths"]["/messages/{message_id}"]["patch"]["parameters"];
        let parameters = parameters.as_array().expect("operation parameters");
        assert!(parameters.iter().any(|p| p["name"] == "Idempotency-Key"));
        assert!(parameters.iter().any(|p| p["name"] == "expand"));
        documents.push(document);
    }
    assert_eq!(documents[0], documents[1]);
    assert_eq!(documents[0], documents[2]);
}
