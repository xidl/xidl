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

const SHORTHAND: &str = r#"
@http(429) exception Limited {
    @header(name = "Retry-After") long delay;
    @cookie("session") string token;
};
interface Messages {
    @patch(path = "/messages/{message_id}{?expand}")
    string edit(@path(path = "message_id") string id,
                @query(name = "expand") @optional string fields,
                @header("Idempotency-Key") @optional string key,
                @cookie(name = "session") @optional string token,
                @header(name = "ETag") out string revision) raises(Limited);
};
"#;

#[test]
fn shorthand_and_rename_generate_the_same_wire_names() {
    for target in ["rust-axum", "typescript-rest", "go-rest"] {
        xidlc::driver::Generator::new(target.to_string())
            .generate_from_idl(SHORTHAND, Path::new("api.idl"), HashMap::new())
            .unwrap_or_else(|error| panic!("{target}: {error}"));
    }
    let mut documents = Vec::new();
    for source in [LEGACY, SHORTHAND] {
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
}
