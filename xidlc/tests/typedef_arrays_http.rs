#![cfg(any(
    feature = "gen-rust-axum",
    feature = "gen-openapi",
    feature = "gen-typescript-rest"
))]

use std::{collections::HashMap, path::Path};

#[test]
#[cfg(feature = "gen-typescript-rest")]
fn typescript_validators_resolve_constant_dimensions_in_the_declaring_scope() {
    let files = xidlc::driver::Generator::new("typescript-rest".into())
        .generate_from_idl(
            include_str!("shared/typedef_arrays_http.idl"),
            Path::new("typedef_arrays.idl"),
            HashMap::new(),
        )
        .expect("constant-sized arrays in TypeScript REST contracts");
    let output = files
        .iter()
        .find(|file| file.path().ends_with("typedef_arrays.zod.ts"))
        .expect("generated Zod validators")
        .content();
    for schema in [
        "PairSchema = z.array(LabelSchema).length(2)",
        "TripleSchema = z.array(LabelSchema).length(3)",
        "CoordinatesSchema = z.array(z.number()).length(2)",
        "BoundsSchema = z.array(z.number()).length(4)",
        "GridSchema = z.array(z.array(PairSchema).length(3)).length(2)",
        "slots: z.array(ImportedPairSchema).length(2)",
    ] {
        assert!(output.contains(schema), "missing {schema}");
    }
    insta::assert_json_snapshot!(
        "typedef_array_zod_contract",
        output.lines().collect::<Vec<_>>()
    );
}

#[test]
#[cfg(feature = "gen-rust-axum")]
fn typedef_arrays_preserve_each_declarator_and_declaring_scope() {
    let files = xidlc::driver::Generator::new("rust-axum".into())
        .generate_from_idl(
            include_str!("shared/typedef_arrays_http.idl"),
            Path::new("typedef_arrays.idl"),
            HashMap::from([
                ("enable_client".into(), true.into()),
                ("enable_server".into(), true.into()),
            ]),
        )
        .expect("legal fixed-array typedefs in REST contracts");
    assert_eq!(files.len(), 1);
    let output = files[0].content();
    for direction in ["in", "out"] {
        let module = format!("__xidl_{direction}_Geo");
        let element = format!("{module}::spatial_Label");
        assert!(output.contains(&format!("pub pair: [{element}; 2]")));
        assert!(output.contains(&format!("pub triple: [{element}; 3]")));
        assert!(output.contains(&format!("pub grid: [[[{element}; 2]; 3]; 2]")));
        assert!(output.contains("pub bounds: [f64; 4]"));
        assert!(output.contains("pub coordinates: Option<[f64; 2]>"));
    }
    insta::assert_snapshot!("typedef_array_http_models", output);
}

#[test]
#[cfg(feature = "gen-openapi")]
fn typedef_array_openapi_preserves_lengths_and_dimension_order() {
    let files = xidlc::driver::Generator::new("openapi".into())
        .generate_from_idl(
            include_str!("shared/typedef_arrays_http.idl"),
            Path::new("typedef_arrays.idl"),
            HashMap::new(),
        )
        .expect("legal fixed-array typedefs in OpenAPI contracts");
    assert_eq!(files.len(), 1);
    let document: serde_json::Value = serde_json::from_str(files[0].content()).unwrap();
    let schemas = &document["components"]["schemas"];
    for (name, length) in [
        ("spatial.Pair", 2),
        ("spatial.Triple", 3),
        ("spatial.Coordinates", 2),
        ("spatial.Bounds", 4),
    ] {
        assert_eq!(schemas[name]["type"], "array");
        assert_eq!(schemas[name]["minItems"], length);
        assert_eq!(schemas[name]["maxItems"], length);
    }
    let grid = &schemas["spatial.Grid"];
    assert_eq!(grid["minItems"], 2);
    assert_eq!(grid["maxItems"], 2);
    assert_eq!(grid["items"]["minItems"], 3);
    assert_eq!(grid["items"]["maxItems"], 3);
    assert_eq!(
        grid["items"]["items"]["$ref"],
        "#/components/schemas/spatial.Pair"
    );
    insta::assert_json_snapshot!("typedef_array_openapi_contract", document);
}
