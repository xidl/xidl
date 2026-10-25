use std::collections::HashMap;
use std::path::Path;
use xidlc::driver::{File, Generator};

const SOURCE: &str = r#"
struct UserRef {
    string id;
};
struct Participant : UserRef {
    string role;
};

@no_security
interface Media {
    @get(path = "/participant")
    Participant participant();
};
"#;

fn generate(lang: &str) -> Vec<File> {
    generate_source(lang, SOURCE)
}

fn generate_source(lang: &str, source: &str) -> Vec<File> {
    let props = HashMap::from([
        ("enable_client".to_string(), true.into()),
        ("enable_server".to_string(), true.into()),
        ("enable_metadata".to_string(), true.into()),
    ]);
    let mut generator = Generator::new(lang.to_string());
    generator
        .generate_from_idl(source, Path::new("struct_inheritance.idl"), props)
        .unwrap_or_else(|error| panic!("generate {lang}: {error}"))
}

fn content(files: &[File], suffix: &str) -> String {
    files
        .iter()
        .find(|file| file.path().ends_with(suffix))
        .unwrap_or_else(|| {
            panic!(
                "missing {suffix}: {:?}",
                files.iter().map(File::path).collect::<Vec<_>>()
            )
        })
        .content()
        .to_string()
}

#[test]
fn every_target_flattens_inherited_struct_fields() {
    let openapi = generate("openapi");
    let document: serde_json::Value =
        serde_json::from_str(&content(&openapi, "openapi_struct_inheritance.json")).expect("json");
    let participant = &document["components"]["schemas"]["Participant"];
    assert!(
        participant["allOf"].as_array().is_some_and(|items| items
            .iter()
            .any(|item| item["$ref"] == "#/components/schemas/UserRef")),
        "OpenAPI must retain the base schema reference: {participant}"
    );

    let typescript = generate("typescript-rest");
    let types = content(&typescript, "struct_inheritance.d.ts");
    assert!(types.contains("export interface Participant extends UserRef"));
    let zod = content(&typescript, "struct_inheritance.zod.ts");
    let participant_schema = zod
        .split("export const ParticipantSchema")
        .nth(1)
        .expect("Participant schema");
    assert!(
        participant_schema.contains("id:"),
        "Zod must validate inherited id: {zod}"
    );
    assert!(
        participant_schema.contains("role:"),
        "Zod must validate own role: {zod}"
    );

    let rust = generate("rust-axum");
    let source = content(&rust, "struct_inheritance.rs");
    assert!(
        source.contains("#[serde(flatten)]\n    pub base: UserRef"),
        "public Rust model must flatten its base: {source}"
    );
    assert!(
        source.contains("#[serde(flatten)]\n        pub base: __xidl_in_Media::UserRef"),
        "HTTP transport must carry the inherited base: {source}"
    );

    let go = generate("go-rest");
    let model = content(&go, "struct_inheritance.go");
    assert!(
        model.contains("type Participant struct {\n\tUserRef\n\tRole string `xjson:\"role\"`"),
        "Go model must embed the base struct: {model}"
    );

    let openrpc = generate("openrpc");
    let document: serde_json::Value =
        serde_json::from_str(&content(&openrpc, "openrpc_struct_inheritance.json")).expect("json");
    let participant = &document["components"]["schemas"]["Participant"];
    assert_eq!(participant["properties"]["id"]["type"], "string");
    assert_eq!(participant["properties"]["role"]["type"], "string");
}

#[test]
fn inherited_schemas_keep_declaring_scopes_even_when_names_are_shadowed() {
    let source = r#"
    typedef string Identifier;
    struct Base { Identifier id; };
    module geo {
        typedef string Identifier;
        struct Identity {
            Identifier id;
            sequence<Identifier> aliases;
            map<string, Identifier> labels;
        };
    };
    module shipping {
        typedef long Identifier;
        module geo { typedef long Identifier; };
        struct RootAddress : ::Base { long priority; };
        struct Address : ::geo::Identity { long priority; };
        @no_security interface Addresses {
            @get(path="/address") Address read();
        };
    };
    // This legal name must not collide with the private root binding.
    module __xidl_root { typedef long Identifier; };
    "#;
    let files = generate_source("typescript-rest", source);
    let models = std::collections::BTreeMap::from([
        ("types", content(&files, "struct_inheritance.d.ts")),
        ("zod", content(&files, "struct_inheritance.zod.ts")),
    ]);
    insta::assert_json_snapshot!("inherited_lexical_scopes", models);
    assert!(models["zod"].contains("id: $xidl_root.IdentifierSchema"));
    assert!(models["zod"].contains("id: $xidl_root.geo.IdentifierSchema"));
    assert!(models["zod"].contains("z.array($xidl_root.geo.IdentifierSchema)"));
    assert!(models["zod"].contains("z.record(z.string(), $xidl_root.geo.IdentifierSchema)"));

    let qualified = source
        .replace(
            "struct Base { Identifier id;",
            "struct Base { ::Identifier id;",
        )
        .replace(
            "            Identifier id;",
            "            ::geo::Identifier id;",
        )
        .replace("sequence<Identifier>", "sequence<::geo::Identifier>")
        .replace("map<string, Identifier>", "map<string, ::geo::Identifier>");
    let qualified_files = generate_source("typescript-rest", &qualified);
    let inherited = |files: &[File]| {
        content(files, "struct_inheritance.zod.ts")
            .split("export namespace shipping")
            .nth(1)
            .expect("shipping schemas")
            .to_string()
    };
    assert_eq!(inherited(&files), inherited(&qualified_files));
}
