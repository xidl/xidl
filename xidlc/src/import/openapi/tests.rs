use super::*;

#[test]
fn test_import_json_schema() {
    let mut dir = std::env::temp_dir();
    dir.push(format!("xidl_test_{}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir_all(&dir).unwrap();

    let input_path = dir.join("schema.json");
    let schema = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "TestSchema",
        "type": "object",
        "properties": {
            "id": { "type": "integer", "format": "int64" },
            "name": { "type": "string", "description": "The name" }
        },
        "required": ["id"],
        "$defs": {
            "Status": {
                "type": "string",
                "enum": ["active", "inactive"]
            }
        }
    });
    fs::write(&input_path, serde_json::to_string(&schema).unwrap()).unwrap();

    import_openapi(&input_path, &dir).unwrap();

    let idl_path = dir.join("schema.idl");
    let idl = fs::read_to_string(idl_path).unwrap();

    assert!(idl.contains("enum Status"));
    assert!(idl.contains("active,"));
    assert!(idl.contains("struct TestSchema"));
    assert!(idl.contains("int64 id;"));
    assert!(idl.contains("/// The name"));
    assert!(idl.contains("@optional"));
    assert!(idl.contains("string name;"));

    fs::remove_dir_all(&dir).unwrap();
}
