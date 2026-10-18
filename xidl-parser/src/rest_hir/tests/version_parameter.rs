//! Location-specific collision coverage for `project_with_options`.

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
        version_parameter: Some(hir::VersionParameter {
            location: hir::HttpParameterLocation::Header,
            name: header.to_string(),
        }),
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
    assert!(error.to_string().contains("@version parameter"), "{error}");
}

#[test]
fn project_with_options_revalidates_transport_names() {
    let spec = lower("interface Store { @get(path=\"/items\") string get(); };");
    let error = project_with_options(&spec, &options("Content-Type"))
        .expect_err("directly supplied version metadata must be validated");
    assert!(
        error
            .to_string()
            .contains("reserved for HTTP transport metadata"),
        "{error}"
    );
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
    assert!(error.to_string().contains("@version parameter"), "{error}");
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
            .contains("same Header wire name 'X-Trace'"),
        "{error}"
    );
}

#[test]
fn collisions_follow_location_case_rules_and_protect_credentials() {
    for (location, kind) in [
        ("header", hir::HttpParameterLocation::Header),
        ("query", hir::HttpParameterLocation::Query),
        ("cookie", hir::HttpParameterLocation::Cookie),
    ] {
        let options = ProjectOptions {
            version_parameter: Some(hir::VersionParameter {
                location: kind,
                name: "Version".into(),
            }),
            version_values: Vec::new(),
        };
        for name in ["Version", "version"] {
            let collision = name == "Version" || location == "header";
            let spec = lower(&format!(
                r#"interface Store {{ @get(path="/items") string get(@{location}(name="{name}") in string value); }};"#
            ));
            assert_eq!(
                project_with_options(&spec, &options).is_err(),
                collision,
                "{location}: {name}"
            );
            let spec = lower(&format!(
                r#"interface Store {{ @get(path="/items") @api_key(in="{location}", name="{name}") string get(); }};"#
            ));
            assert_eq!(
                project_with_options(&spec, &options).is_err(),
                collision,
                "credential {location}: {name}"
            );
        }
        let other = if location == "header" {
            "query"
        } else {
            "header"
        };
        let spec = lower(&format!(
            r#"interface Store {{ @get(path="/items") string get(@{other} @rename("Version") in string value); }};"#
        ));
        assert!(
            project_with_options(&spec, &options).is_ok(),
            "names at different locations are independent"
        );
    }
}

#[test]
fn version_headers_cannot_overwrite_business_response_metadata() {
    for source in [
        r#"interface Store { @get(path="/items") void get(@header @rename("x-VERSION") out string etag); };"#,
        r#"@http(status=409) exception Conflict { @header @rename("X-version") string revision; }; interface Store { @get(path="/items") string get() raises(Conflict); };"#,
        r#"@http(status=409) exception Conflict { @header("X-version") string revision; }; interface Store { @get(path="/items") string get() raises(Conflict); };"#,
    ] {
        let spec = lower(source);
        let error = project_with_options(&spec, &options("X-Version"))
            .expect_err("fixed version header must not overwrite application data");
        assert!(
            error
                .to_string()
                .contains("conflicts with the fixed @version header"),
            "{error}"
        );
        assert!(project_with_options(&spec, &options("X-Api-Version")).is_ok());
        for location in [
            hir::HttpParameterLocation::Query,
            hir::HttpParameterLocation::Cookie,
        ] {
            let options = ProjectOptions {
                version_parameter: Some(hir::VersionParameter {
                    location,
                    name: "X-Version".into(),
                }),
                version_values: Vec::new(),
            };
            assert!(project_with_options(&spec, &options).is_ok());
        }
    }
}

#[test]
fn version_parameter_protects_effective_authorization_credentials() {
    for scheme in ["http_basic", "http_bearer"] {
        for source in [
            format!("@{scheme} interface Store {{ @get(path=\"/items\") string get(); }};"),
            format!("interface Store {{ @{scheme} @get(path=\"/items\") string get(); }};"),
        ] {
            let spec = lower(&source);
            let error = project_with_options(&spec, &options("aUtHoRiZaTiOn"))
                .expect_err("version must not overwrite effective authentication");
            assert!(
                error.to_string().contains("overwrite the credential"),
                "{error}"
            );
        }
        let spec = lower(&format!(
            "@{scheme} interface Store {{ @no_security @get(path=\"/items\") string get(); }};"
        ));
        assert!(project_with_options(&spec, &options("Authorization")).is_ok());
    }
}

#[test]
fn version_cookie_cannot_rewrite_an_opaque_cookie_header() {
    let options = ProjectOptions {
        version_parameter: Some(hir::VersionParameter {
            location: hir::HttpParameterLocation::Cookie,
            name: "version".into(),
        }),
        version_values: Vec::new(),
    };
    for source in [
        r#"@api_key(in="header", name="cOOkie") interface Store { @get(path="/items") string get(); };"#,
        r#"interface Store { @get(path="/items") string get(@header @rename("COOKIE") in string raw); };"#,
    ] {
        let error = project_with_options(&lower(source), &options)
            .expect_err("a version cookie would rewrite the whole-header binding");
        assert!(error.to_string().contains("@version parameter"), "{error}");
    }
    let spec = lower(
        r#"@api_key(in="cookie", name="session") interface Store { @get(path="/items") string get(@cookie in string theme); };"#,
    );
    assert!(
        project_with_options(&spec, &options).is_ok(),
        "independently named cookies coexist"
    );
}
