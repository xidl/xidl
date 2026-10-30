use crate::hir;
use crate::rest_hir;

fn parser_result(source: &str) -> Result<rest_hir::RestHirDocument, crate::error::ParseError> {
    let typed = crate::parser::parser_text(source).expect("parse idl");
    let spec = hir::Specification::from(typed);
    rest_hir::project(&spec)
}

#[test]
fn rejects_one_interface_binding_the_same_method_and_path_twice() {
    let error = parser_result(
        r#"
        interface DirectoryApi {
          @get(path = "/users/{id}")
          string get_user(@path @rename("id") string id);

          @get(path = "/users/{id}")
          string fetch_user(@path @rename("id") string id);
        };
        "#,
    )
    .expect_err("a repeated route binding should fail projection");
    let message = error.to_string();
    assert!(
        message.contains("duplicate HTTP route binding in interface 'DirectoryApi'"),
        "{message}"
    );
    assert!(message.contains("GET /users/{id}"), "{message}");
    assert!(message.contains("get_user, fetch_user"), "{message}");
}

#[test]
fn accepts_distinct_paths_and_distinct_methods() {
    parser_result(
        r#"
        interface DirectoryApi {
          @get(path = "/users/{id}")
          string get_user(@path @rename("id") string id);

          @get(path = "/items/{id}")
          string get_item(@path @rename("id") string id);

          @delete(path = "/users/{id}")
          void delete_user(@path @rename("id") string id);
        };
        "#,
    )
    .expect("distinct bindings should project");
}

#[test]
fn accepts_one_route_in_two_interfaces() {
    parser_result(
        r#"
        interface DirectoryApi {
          @get(path = "/users/{id}")
          string get_user(@path @rename("id") string id);
        };

        interface LegacyDirectoryApi {
          @get(path = "/users/{id}")
          string get_legacy_user(@path @rename("id") string id);
        };
        "#,
    )
    .expect("interfaces own separate route tables");
}
