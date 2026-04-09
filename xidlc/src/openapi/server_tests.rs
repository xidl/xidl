use crate::openapi::{ServerBuilder, ServerVariableBuilder};

macro_rules! test_fn {
    ($name:ident: $schema:expr; $expected:literal) => {
        #[test]
        fn $name() {
            let value = serde_json::to_value($schema).unwrap();
            let expected_value: serde_json::Value = serde_json::from_str($expected).unwrap();
            assert_eq!(
                value,
                expected_value,
                "testing serializing \"{}\": \nactual:\n{}\nexpected:\n{}",
                stringify!($name),
                value,
                expected_value
            );
            println!("{}", &serde_json::to_string_pretty(&$schema).unwrap());
        }
    };
}

test_fn! {
create_server_with_builder_and_variable_substitution:
ServerBuilder::new().url("/api/{version}/{username}")
    .parameter("version", ServerVariableBuilder::new()
        .enum_values(Some(["v1", "v2"]))
        .description(Some("api version"))
        .default_value("v1"))
    .parameter("username", ServerVariableBuilder::new()
        .default_value("the_user")).build();
r###"{
  "url": "/api/{version}/{username}",
  "variables": {
      "version": {
          "enum": ["v1", "v2"],
          "default": "v1",
          "description": "api version"
      },
      "username": {
          "default": "the_user"
      }
  }
}"###
}
