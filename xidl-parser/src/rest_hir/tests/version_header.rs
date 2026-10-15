//! Header-collision coverage for `project_with_options`.

use std::collections::HashMap;

use crate::hir;

use crate::rest_hir::{ProjectOptions, project, project_with_options};

fn lower(source: &str) -> hir::Specification {
    let typed = crate::parser::parser_text(source).expect("parse idl");
    hir::Specification::from_typed_ast_with_properties_and_path(typed, HashMap::new(), "input.idl")
        .expect("lower")
}

fn options(header: &str) -> ProjectOptions {
    ProjectOptions {
        version_header: Some(header.to_string()),
        version_values: Vec::new(),
    }
}

#[test]
fn version_header_collision_with_header_parameter_is_rejected() {
    let spec = lower(
        r#"
        interface Store {
            @post(path="/items")
            string create(@header @rename("X-Api-Version") in string item, in string body);
        };
        "#,
    );
    let error = project_with_options(&spec, &options("X-Api-Version"))
        .expect_err("version header collision");
    assert!(error.to_string().contains("@version header"), "{error}");
}

#[test]
fn version_header_collision_is_case_insensitive() {
    let spec = lower(
        r#"
        interface Store {
            @post(path="/items")
            string create(@header @rename("x-aPi-vErsION") in string item, in string body);
        };
        "#,
    );
    let error = project_with_options(&spec, &options("X-Api-Version"))
        .expect_err("case-insensitive collision");
    assert!(
        error.to_string().contains("rename the parameter"),
        "{error}"
    );
}

#[test]
fn api_key_header_scheme_colliding_with_version_header_is_rejected() {
    let spec = lower(
        r#"
        interface Store {
            @api_key(in="header", name="X-Api-Version")
            @post(path="/items")
            string create(in string body);
        };
        "#,
    );
    let error =
        project_with_options(&spec, &options("X-Api-Version")).expect_err("api key collision");
    assert!(
        error.to_string().contains("overwrite the credential"),
        "{error}"
    );
}

#[test]
fn duplicate_case_insensitive_header_params_are_rejected() {
    let spec = lower(
        r#"
        interface Store {
            @post(path="/items")
            string create(@header @rename("X-Trace") in string trace, @header @rename("x-trace") in string other);
        };
        "#,
    );
    let error = project(&spec).expect_err("duplicate header names");
    assert!(
        error
            .to_string()
            .contains("case-insensitive name 'X-Trace'"),
        "{error}"
    );
}
