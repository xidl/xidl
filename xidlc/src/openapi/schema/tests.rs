use insta::assert_json_snapshot;
use serde_json::{Value, json};

use super::*;
use crate::openapi::*;

#[test]
fn create_schema_serializes_json() -> Result<(), serde_json::Error> {
    let openapi = OpenApiBuilder::new()
        .info(Info::new("My api", "1.0.0"))
        .paths(Paths::new())
        .components(Some(
            ComponentsBuilder::new()
                .schema("Person", Ref::new("#/components/PersonModel"))
                .schema(
                    "Credential",
                    Schema::from(
                        ObjectBuilder::new()
                            .property(
                                "id",
                                ObjectBuilder::new()
                                    .schema_type(Type::Integer)
                                    .format(Some(SchemaFormat::KnownFormat(KnownFormat::Int32)))
                                    .description(Some("Id of credential"))
                                    .default(Some(json!(1i32))),
                            )
                            .property(
                                "name",
                                ObjectBuilder::new()
                                    .schema_type(Type::String)
                                    .description(Some("Name of credential")),
                            )
                            .property(
                                "status",
                                ObjectBuilder::new()
                                    .schema_type(Type::String)
                                    .default(Some(json!("Active")))
                                    .description(Some("Credential status"))
                                    .enum_values(Some([
                                        "Active",
                                        "NotActive",
                                        "Locked",
                                        "Expired",
                                    ])),
                            )
                            .property(
                                "history",
                                Array::new(Ref::from_schema_name("UpdateHistory")),
                            )
                            .property("tags", Object::with_type(Type::String).to_array()),
                    ),
                )
                .build(),
        ))
        .build();

    let serialized = serde_json::to_string_pretty(&openapi)?;
    println!("serialized json:\n {serialized}");

    let value = serde_json::to_value(&openapi)?;
    let credential = get_json_path(&value, "components.schemas.Credential.properties");
    let person = get_json_path(&value, "components.schemas.Person");

    assert!(
        credential.get("id").is_some(),
        "could not find path: components.schemas.Credential.properties.id"
    );
    assert!(
        credential.get("status").is_some(),
        "could not find path: components.schemas.Credential.properties.status"
    );
    assert!(
        credential.get("name").is_some(),
        "could not find path: components.schemas.Credential.properties.name"
    );
    assert!(
        credential.get("history").is_some(),
        "could not find path: components.schemas.Credential.properties.history"
    );
    assert_eq!(
        credential
            .get("id")
            .unwrap_or(&serde_json::value::Value::Null)
            .to_string(),
        r#"{"default":1,"description":"Id of credential","format":"int32","type":"integer"}"#,
        "components.schemas.Credential.properties.id did not match"
    );
    assert_eq!(
        credential
            .get("name")
            .unwrap_or(&serde_json::value::Value::Null)
            .to_string(),
        r#"{"description":"Name of credential","type":"string"}"#,
        "components.schemas.Credential.properties.name did not match"
    );
    assert_eq!(
        credential
            .get("status")
            .unwrap_or(&serde_json::value::Value::Null)
            .to_string(),
        r#"{"default":"Active","description":"Credential status","enum":["Active","NotActive","Locked","Expired"],"type":"string"}"#,
        "components.schemas.Credential.properties.status did not match"
    );
    assert_eq!(
        credential
            .get("history")
            .unwrap_or(&serde_json::value::Value::Null)
            .to_string(),
        r###"{"items":{"$ref":"#/components/schemas/UpdateHistory"},"type":"array"}"###,
        "components.schemas.Credential.properties.history did not match"
    );
    assert_eq!(
        person.to_string(),
        r###"{"$ref":"#/components/PersonModel"}"###,
        "components.schemas.Person.ref did not match"
    );

    Ok(())
}

// Examples taken from https://spec.openapis.org/oas/latest.html#model-with-map-dictionary-properties
#[test]
fn test_property_order() {
    let json_value = ObjectBuilder::new()
        .property(
            "id",
            ObjectBuilder::new()
                .schema_type(Type::Integer)
                .format(Some(SchemaFormat::KnownFormat(KnownFormat::Int32)))
                .description(Some("Id of credential"))
                .default(Some(json!(1i32))),
        )
        .property(
            "name",
            ObjectBuilder::new()
                .schema_type(Type::String)
                .description(Some("Name of credential")),
        )
        .property(
            "status",
            ObjectBuilder::new()
                .schema_type(Type::String)
                .default(Some(json!("Active")))
                .description(Some("Credential status"))
                .enum_values(Some(["Active", "NotActive", "Locked", "Expired"])),
        )
        .property(
            "history",
            Array::new(Ref::from_schema_name("UpdateHistory")),
        )
        .property("tags", Object::with_type(Type::String).to_array())
        .build();

    assert_eq!(
        json_value.properties.keys().collect::<Vec<_>>(),
        vec!["history", "id", "name", "status", "tags"]
    );
}

// Examples taken from https://spec.openapis.org/oas/latest.html#model-with-map-dictionary-properties
#[test]
fn test_additional_properties() {
    let json_value = ObjectBuilder::new()
        .additional_properties(Some(ObjectBuilder::new().schema_type(Type::String)))
        .build();
    assert_json_snapshot!(json_value, @r#"
    {
      "type": "object",
      "additionalProperties": {
        "type": "string"
      }
    }
    "#);

    let json_value = ObjectBuilder::new()
        .additional_properties(Some(ArrayBuilder::new().items(ArrayItems::RefOrSchema(
            Box::new(ObjectBuilder::new().schema_type(Type::Number).into()),
        ))))
        .build();
    assert_json_snapshot!(json_value, @r#"
    {
      "type": "object",
      "additionalProperties": {
        "type": "array",
        "items": {
          "type": "number"
        }
      }
    }
    "#);

    let json_value = ObjectBuilder::new()
        .additional_properties(Some(Ref::from_schema_name("ComplexModel")))
        .build();
    assert_json_snapshot!(json_value, @r##"
    {
      "type": "object",
      "additionalProperties": {
        "$ref": "#/components/schemas/ComplexModel"
      }
    }
    "##);
}

#[test]
fn test_object_with_title() {
    let json_value = ObjectBuilder::new().title(Some("SomeName")).build();
    assert_json_snapshot!(json_value, @r#"
    {
      "type": "object",
      "title": "SomeName"
    }
    "#);
}

#[test]
fn derive_object_with_examples() {
    let json_value = ObjectBuilder::new()
        .examples([Some(json!({"age": 20, "name": "bob the cat"}))])
        .build();
    assert_json_snapshot!(json_value, @r#"
    {
      "type": "object",
      "examples": [
        {
          "age": 20,
          "name": "bob the cat"
        }
      ]
    }
    "#);
}

fn get_json_path<'a>(value: &'a Value, path: &str) -> &'a Value {
    path.split('.').fold(value, |acc, fragment| {
        acc.get(fragment).unwrap_or(&serde_json::value::Value::Null)
    })
}

#[test]
fn test_array_new() {
    let array = Array::new(
        ObjectBuilder::new().property(
            "id",
            ObjectBuilder::new()
                .schema_type(Type::Integer)
                .format(Some(SchemaFormat::KnownFormat(KnownFormat::Int32)))
                .description(Some("Id of credential"))
                .default(Some(json!(1i32))),
        ),
    );

    assert!(matches!(array.schema_type, SchemaType::Type(Type::Array)));
}

#[test]
fn test_array_builder() {
    let array: Array = ArrayBuilder::new()
        .items(
            ObjectBuilder::new().property(
                "id",
                ObjectBuilder::new()
                    .schema_type(Type::Integer)
                    .format(Some(SchemaFormat::KnownFormat(KnownFormat::Int32)))
                    .description(Some("Id of credential"))
                    .default(Some(json!(1i32))),
            ),
        )
        .build();

    assert!(matches!(array.schema_type, SchemaType::Type(Type::Array)));
}

#[test]
fn reserialize_deserialized_schema_components() {
    let components = ComponentsBuilder::new()
        .schemas_from_iter(vec![(
            "Comp",
            Schema::from(
                ObjectBuilder::new()
                    .property("name", ObjectBuilder::new().schema_type(Type::String))
                    .required("name"),
            ),
        )])
        .responses_from_iter(vec![(
            "200",
            ResponseBuilder::new().description("Okay").build(),
        )])
        .security_scheme(
            "TLS",
            SecurityScheme::MutualTls {
                description: None,
                extensions: None,
            },
        )
        .build();

    let serialized_components = serde_json::to_string(&components).unwrap();

    let deserialized_components: Components =
        serde_json::from_str(serialized_components.as_str()).unwrap();

    assert_eq!(
        serialized_components,
        serde_json::to_string(&deserialized_components).unwrap()
    )
}

#[test]
fn reserialize_deserialized_object_component() {
    let prop = ObjectBuilder::new()
        .property("name", ObjectBuilder::new().schema_type(Type::String))
        .required("name")
        .build();

    let serialized_components = serde_json::to_string(&prop).unwrap();
    let deserialized_components: Object =
        serde_json::from_str(serialized_components.as_str()).unwrap();

    assert_eq!(
        serialized_components,
        serde_json::to_string(&deserialized_components).unwrap()
    )
}

#[test]
fn reserialize_deserialized_property() {
    let prop = ObjectBuilder::new().schema_type(Type::String).build();

    let serialized_components = serde_json::to_string(&prop).unwrap();
    let deserialized_components: Object =
        serde_json::from_str(serialized_components.as_str()).unwrap();

    assert_eq!(
        serialized_components,
        serde_json::to_string(&deserialized_components).unwrap()
    )
}

#[test]
fn serialize_deserialize_array_within_ref_or_t_object_builder() {
    let ref_or_schema = RefOr::T(Schema::Object(
        ObjectBuilder::new()
            .property(
                "test",
                RefOr::T(Schema::Array(
                    ArrayBuilder::new()
                        .items(RefOr::T(Schema::Object(
                            ObjectBuilder::new()
                                .property("element", RefOr::Ref(Ref::new("#/test")))
                                .build(),
                        )))
                        .build(),
                )),
            )
            .build(),
    ));

    let json_str = serde_json::to_string(&ref_or_schema).expect("");
    println!("----------------------------");
    println!("{json_str}");

    let deserialized: RefOr<Schema> = serde_json::from_str(&json_str).expect("");

    let json_de_str = serde_json::to_string(&deserialized).expect("");
    println!("----------------------------");
    println!("{json_de_str}");

    assert_eq!(json_str, json_de_str);
}

#[test]
fn serialize_deserialize_one_of_within_ref_or_t_object_builder() {
    let ref_or_schema = RefOr::T(Schema::Object(
        ObjectBuilder::new()
            .property(
                "test",
                RefOr::T(Schema::OneOf(
                    OneOfBuilder::new()
                        .item(Schema::Array(
                            ArrayBuilder::new()
                                .items(RefOr::T(Schema::Object(
                                    ObjectBuilder::new()
                                        .property("element", RefOr::Ref(Ref::new("#/test")))
                                        .build(),
                                )))
                                .build(),
                        ))
                        .item(Schema::Array(
                            ArrayBuilder::new()
                                .items(RefOr::T(Schema::Object(
                                    ObjectBuilder::new()
                                        .property("foobar", RefOr::Ref(Ref::new("#/foobar")))
                                        .build(),
                                )))
                                .build(),
                        ))
                        .build(),
                )),
            )
            .build(),
    ));

    let json_str = serde_json::to_string(&ref_or_schema).expect("");
    println!("----------------------------");
    println!("{json_str}");

    let deserialized: RefOr<Schema> = serde_json::from_str(&json_str).expect("");

    let json_de_str = serde_json::to_string(&deserialized).expect("");
    println!("----------------------------");
    println!("{json_de_str}");

    assert_eq!(json_str, json_de_str);
}

#[test]
fn serialize_deserialize_all_of_of_within_ref_or_t_object_builder() {
    let ref_or_schema = RefOr::T(Schema::Object(
        ObjectBuilder::new()
            .property(
                "test",
                RefOr::T(Schema::AllOf(
                    AllOfBuilder::new()
                        .item(Schema::Array(
                            ArrayBuilder::new()
                                .items(RefOr::T(Schema::Object(
                                    ObjectBuilder::new()
                                        .property("element", RefOr::Ref(Ref::new("#/test")))
                                        .build(),
                                )))
                                .build(),
                        ))
                        .item(RefOr::T(Schema::Object(
                            ObjectBuilder::new()
                                .property("foobar", RefOr::Ref(Ref::new("#/foobar")))
                                .build(),
                        )))
                        .build(),
                )),
            )
            .build(),
    ));

    let json_str = serde_json::to_string(&ref_or_schema).expect("");
    println!("----------------------------");
    println!("{json_str}");

    let deserialized: RefOr<Schema> = serde_json::from_str(&json_str).expect("");

    let json_de_str = serde_json::to_string(&deserialized).expect("");
    println!("----------------------------");
    println!("{json_de_str}");

    assert_eq!(json_str, json_de_str);
}

#[test]
fn deserialize_reserialize_one_of_default_type() {
    let a = OneOfBuilder::new()
        .item(Schema::Array(
            ArrayBuilder::new()
                .items(RefOr::T(Schema::Object(
                    ObjectBuilder::new()
                        .property("element", RefOr::Ref(Ref::new("#/test")))
                        .build(),
                )))
                .build(),
        ))
        .item(Schema::Array(
            ArrayBuilder::new()
                .items(RefOr::T(Schema::Object(
                    ObjectBuilder::new()
                        .property("foobar", RefOr::Ref(Ref::new("#/foobar")))
                        .build(),
                )))
                .build(),
        ))
        .build();

    let serialized_json = serde_json::to_string(&a).expect("should serialize to json");
    let b: OneOf = serde_json::from_str(&serialized_json).expect("should deserialize OneOf");
    let reserialized_json = serde_json::to_string(&b).expect("reserialized json");

    println!("{serialized_json}");
    println!("{reserialized_json}",);
    assert_eq!(serialized_json, reserialized_json);
}

#[test]
fn serialize_deserialize_any_of_of_within_ref_or_t_object_builder() {
    let ref_or_schema = RefOr::T(Schema::Object(
        ObjectBuilder::new()
            .property(
                "test",
                RefOr::T(Schema::AnyOf(
                    AnyOfBuilder::new()
                        .item(Schema::Array(
                            ArrayBuilder::new()
                                .items(RefOr::T(Schema::Object(
                                    ObjectBuilder::new()
                                        .property("element", RefOr::Ref(Ref::new("#/test")))
                                        .build(),
                                )))
                                .build(),
                        ))
                        .item(RefOr::T(Schema::Object(
                            ObjectBuilder::new()
                                .property("foobar", RefOr::Ref(Ref::new("#/foobar")))
                                .build(),
                        )))
                        .build(),
                )),
            )
            .build(),
    ));

    let json_str = serde_json::to_string(&ref_or_schema).expect("");
    println!("----------------------------");
    println!("{json_str}");

    let deserialized: RefOr<Schema> = serde_json::from_str(&json_str).expect("");

    let json_de_str = serde_json::to_string(&deserialized).expect("");
    println!("----------------------------");
    println!("{json_de_str}");
    assert!(json_str.contains("\"anyOf\""));
    assert_eq!(json_str, json_de_str);
}

#[test]
fn serialize_deserialize_schema_array_ref_or_t() {
    let ref_or_schema = RefOr::T(Schema::Array(
        ArrayBuilder::new()
            .items(RefOr::T(Schema::Object(
                ObjectBuilder::new()
                    .property("element", RefOr::Ref(Ref::new("#/test")))
                    .build(),
            )))
            .build(),
    ));

    let json_str = serde_json::to_string(&ref_or_schema).expect("");
    println!("----------------------------");
    println!("{json_str}");

    let deserialized: RefOr<Schema> = serde_json::from_str(&json_str).expect("");

    let json_de_str = serde_json::to_string(&deserialized).expect("");
    println!("----------------------------");
    println!("{json_de_str}");

    assert_eq!(json_str, json_de_str);
}

#[test]
fn serialize_deserialize_schema_array_builder() {
    let ref_or_schema = ArrayBuilder::new()
        .items(RefOr::T(Schema::Object(
            ObjectBuilder::new()
                .property("element", RefOr::Ref(Ref::new("#/test")))
                .build(),
        )))
        .build();

    let json_str = serde_json::to_string(&ref_or_schema).expect("");
    println!("----------------------------");
    println!("{json_str}");

    let deserialized: RefOr<Schema> = serde_json::from_str(&json_str).expect("");

    let json_de_str = serde_json::to_string(&deserialized).expect("");
    println!("----------------------------");
    println!("{json_de_str}");

    assert_eq!(json_str, json_de_str);
}

#[test]
fn serialize_deserialize_schema_with_additional_properties() {
    let schema = Schema::Object(
        ObjectBuilder::new()
            .property(
                "map",
                ObjectBuilder::new()
                    .additional_properties(Some(AdditionalProperties::FreeForm(true))),
            )
            .build(),
    );

    let json_str = serde_json::to_string(&schema).unwrap();
    println!("----------------------------");
    println!("{json_str}");

    let deserialized: RefOr<Schema> = serde_json::from_str(&json_str).unwrap();

    let json_de_str = serde_json::to_string(&deserialized).unwrap();
    println!("----------------------------");
    println!("{json_de_str}");

    assert_eq!(json_str, json_de_str);
}

#[test]
fn serialize_deserialize_schema_with_additional_properties_object() {
    let schema = Schema::Object(
        ObjectBuilder::new()
            .property(
                "map",
                ObjectBuilder::new().additional_properties(Some(
                    ObjectBuilder::new().property("name", Object::with_type(Type::String)),
                )),
            )
            .build(),
    );

    let json_str = serde_json::to_string(&schema).unwrap();
    println!("----------------------------");
    println!("{json_str}");

    let deserialized: RefOr<Schema> = serde_json::from_str(&json_str).unwrap();

    let json_de_str = serde_json::to_string(&deserialized).unwrap();
    println!("----------------------------");
    println!("{json_de_str}");

    assert_eq!(json_str, json_de_str);
}

#[test]
fn serialize_discriminator_with_mapping() {
    let mut discriminator = Discriminator::new("type");
    discriminator.mapping = [("int".to_string(), "#/components/schemas/MyInt".to_string())]
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    let one_of = OneOfBuilder::new()
        .item(Ref::from_schema_name("MyInt"))
        .discriminator(Some(discriminator))
        .build();
    assert_json_snapshot!(one_of, @r##"
    {
      "oneOf": [
        {
          "$ref": "#/components/schemas/MyInt"
        }
      ],
      "discriminator": {
        "propertyName": "type",
        "mapping": {
          "int": "#/components/schemas/MyInt"
        }
      }
    }
    "##);
}

#[test]
fn serialize_deserialize_object_with_multiple_schema_types() {
    let object = ObjectBuilder::new()
        .schema_type(SchemaType::from_iter([Type::Object, Type::Null]))
        .build();

    let json_str = serde_json::to_string(&object).unwrap();
    println!("----------------------------");
    println!("{json_str}");

    let deserialized: Object = serde_json::from_str(&json_str).unwrap();

    let json_de_str = serde_json::to_string(&deserialized).unwrap();
    println!("----------------------------");
    println!("{json_de_str}");

    assert_eq!(json_str, json_de_str);
}

#[test]
fn object_with_extensions() {
    let expected = json!("value");
    let extensions = extensions::ExtensionsBuilder::new()
        .add("x-some-extension", expected.clone())
        .build();
    let json_value = ObjectBuilder::new().extensions(Some(extensions)).build();

    let value = serde_json::to_value(&json_value).unwrap();
    assert_eq!(value.get("x-some-extension"), Some(&expected));
}

#[test]
fn array_with_extensions() {
    let expected = json!("value");
    let extensions = extensions::ExtensionsBuilder::new()
        .add("x-some-extension", expected.clone())
        .build();
    let json_value = ArrayBuilder::new().extensions(Some(extensions)).build();

    let value = serde_json::to_value(&json_value).unwrap();
    assert_eq!(value.get("x-some-extension"), Some(&expected));
}

#[test]
fn oneof_with_extensions() {
    let expected = json!("value");
    let extensions = extensions::ExtensionsBuilder::new()
        .add("x-some-extension", expected.clone())
        .build();
    let json_value = OneOfBuilder::new().extensions(Some(extensions)).build();

    let value = serde_json::to_value(&json_value).unwrap();
    assert_eq!(value.get("x-some-extension"), Some(&expected));
}

#[test]
fn allof_with_extensions() {
    let expected = json!("value");
    let extensions = extensions::ExtensionsBuilder::new()
        .add("x-some-extension", expected.clone())
        .build();
    let json_value = AllOfBuilder::new().extensions(Some(extensions)).build();

    let value = serde_json::to_value(&json_value).unwrap();
    assert_eq!(value.get("x-some-extension"), Some(&expected));
}

#[test]
fn anyof_with_extensions() {
    let expected = json!("value");
    let extensions = extensions::ExtensionsBuilder::new()
        .add("x-some-extension", expected.clone())
        .build();
    let json_value = AnyOfBuilder::new().extensions(Some(extensions)).build();

    let value = serde_json::to_value(&json_value).unwrap();
    assert_eq!(value.get("x-some-extension"), Some(&expected));
}
