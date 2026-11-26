use std::collections::HashMap;
use std::path::Path;

#[test]
fn every_target_rejects_bare_enum_labels_before_rendering() {
    let source = include_str!("../../bdd/features/data/union_unqualified_case.idl");
    let targets = [
        "hir",
        "rest-hir",
        "version-map",
        #[cfg(feature = "gen-rust")]
        "rust",
        #[cfg(feature = "gen-rust-axum")]
        "rust-axum",
        #[cfg(feature = "gen-rust-axum-adapters")]
        "rust-axum-adapters",
        #[cfg(feature = "gen-typescript")]
        "typescript",
        #[cfg(feature = "gen-typescript-rest")]
        "typescript-rest",
        #[cfg(feature = "gen-typescript-rest-adapters")]
        "typescript-rest-adapters",
        #[cfg(feature = "gen-go")]
        "go",
        #[cfg(feature = "gen-go-rest")]
        "go-rest",
        #[cfg(feature = "gen-openapi")]
        "openapi",
    ];
    for target in targets {
        let result = xidlc::driver::Generator::new(target.into()).generate_from_idl(
            source,
            Path::new("union_unqualified_case.idl"),
            HashMap::new(),
        );
        let error = match result {
            Ok(_) => panic!("{target} accepted a bare enum label"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("union 'Payload'"), "{target}: {error}");
        assert!(error.contains("case label 'mms'"), "{target}: {error}");
        assert!(error.contains("Enum::member"), "{target}: {error}");
    }
}

#[cfg(feature = "gen-rust")]
#[test]
fn documented_union_uses_qualified_labels_and_struct_payloads() {
    let guide = include_str!("../../website/src/content/docs/docs/index.mdx");
    let section = guide.split("## Union").nth(1).expect("union guide");
    let source = section
        .split("```idl")
        .nth(1)
        .expect("union IDL example")
        .split("```")
        .next()
        .expect("IDL fence");
    let files = xidlc::driver::Generator::new("rust".into())
        .generate_from_idl(source, Path::new("color_point.idl"), HashMap::new())
        .expect("the documented example passes shared source validation");
    let output = files
        .iter()
        .map(|file| file.content())
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!("documented_color_point", output);
}

#[test]
fn renamed_enum_discriminator_propagates_to_union_tags_across_targets() {
    let model_source = r#"
        enum Kind { @rename("kind-a") alpha, beta };
        struct A { string x; };
        union U switch (Kind) {
            case Kind::alpha: A first;
            case Kind::beta: @rename("second-wire") A second;
        };
        struct Holder { U u; Kind k; };
    "#;
    let http_source =
        format!("{model_source}\ninterface Api {{ @get(path = \"/holder\") Holder get(); }};");

    let render = |target: &str, source: &str| {
        xidlc::driver::Generator::new(target.into())
            .generate_from_idl(source, Path::new("t.idl"), HashMap::new())
            .unwrap_or_else(|error| panic!("{target} failed: {error}"))
            .into_iter()
            .map(|file| file.content().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    };

    #[cfg(feature = "gen-typescript")]
    {
        let ts = render("typescript", model_source);
        assert!(ts.contains("\"kind-a\" | \"beta\""), "{ts}");
        assert!(ts.contains("tag: \"kind-a\""), "{ts}");
        assert!(ts.contains("tag: \"beta\""), "{ts}");
        assert!(ts.contains("z.enum([\"kind-a\"])"), "{ts}");
        assert!(ts.contains("z.enum([\"beta\"])"), "{ts}");
        assert!(!ts.contains("tag: \"alpha\""), "{ts}");
        assert!(!ts.contains("z.enum([\"alpha\"])"), "{ts}");
    }

    #[cfg(feature = "gen-openapi")]
    {
        let oa = render("openapi", &http_source);
        let doc: serde_json::Value = serde_json::from_str(&oa).expect("valid openapi json");
        let schemas = &doc["components"]["schemas"];
        assert_eq!(
            schemas["Kind"]["enum"],
            serde_json::json!(["kind-a", "beta"])
        );
        let branches = schemas["U"]["oneOf"].as_array().expect("U oneOf");
        assert_eq!(
            branches[0]["properties"]["tag"]["enum"],
            serde_json::json!(["kind-a"])
        );
        assert_eq!(
            branches[1]["properties"]["tag"]["enum"],
            serde_json::json!(["beta"])
        );
    }

    #[cfg(feature = "gen-rust")]
    {
        let rs = render("rust", model_source);
        assert!(rs.contains("\"kind-a\" =>"), "{rs}");
        assert!(rs.contains("let tag = \"kind-a\""), "{rs}");
        assert!(rs.contains("\"beta\" =>"), "{rs}");
        assert!(rs.contains("let tag = \"beta\""), "{rs}");
        assert!(!rs.contains("\"alpha\" =>"), "{rs}");
        assert!(!rs.contains("let tag = \"alpha\""), "{rs}");
    }

    #[cfg(feature = "gen-go")]
    {
        let go = render("go", model_source);
        assert!(go.contains("tag: \"kind-a\""), "{go}");
        assert!(go.contains("case \"kind-a\":"), "{go}");
        assert!(go.contains("tag: \"beta\""), "{go}");
        assert!(go.contains("case \"beta\":"), "{go}");
        assert!(!go.contains("tag: \"alpha\""), "{go}");
        assert!(!go.contains("case \"alpha\":"), "{go}");
    }
}

#[test]
fn renamed_enum_discriminator_propagates_to_version_adapters() {
    let source = r#"
        @version enum V { v1, v2 };
        enum Kind { @rename("kind-a") alpha, beta };
        struct A { string x; @added(V::v2) @optional string y; };
        union U switch (Kind) {
            case Kind::alpha: A first;
            case Kind::beta: A second;
        };
        interface Api {
            @post(path = "/submit")
            U submit(U value);
        };
    "#;

    #[cfg(feature = "gen-typescript-rest-adapters")]
    {
        let files = xidlc::driver::Generator::new("typescript-rest-adapters".into())
            .generate_from_idl(source, Path::new("t.idl"), HashMap::new())
            .expect("ts adapters");
        let ts = files
            .iter()
            .map(|f| f.content())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(ts.contains("z.enum([\"kind-a\"])"), "{ts}");
        assert!(ts.contains("case \"kind-a\":"), "{ts}");
        assert!(ts.contains("case \"beta\":"), "{ts}");
        assert!(!ts.contains("\"alpha\""), "{ts}");
    }

    #[cfg(feature = "gen-rust-axum-adapters")]
    {
        let files = xidlc::driver::Generator::new("rust-axum-adapters".into())
            .generate_from_idl(source, Path::new("t.idl"), HashMap::new())
            .expect("rust adapters");
        let rs = files
            .iter()
            .map(|f| f.content())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rs.contains("\"kind-a\""), "{rs}");
        assert!(rs.contains("\"beta\""), "{rs}");
        assert!(!rs.contains("\"alpha\""), "{rs}");
    }
}
