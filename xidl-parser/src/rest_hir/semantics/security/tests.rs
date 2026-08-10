use super::*;
use crate::hir::{Annotation, AnnotationParams};

fn builtin(name: &str, raw: &str) -> Annotation {
    Annotation::Builtin {
        name: name.to_string(),
        params: Some(AnnotationParams::Raw(raw.to_string())),
    }
}

fn bare(name: &str) -> Annotation {
    Annotation::Builtin {
        name: name.to_string(),
        params: None,
    }
}

#[test]
fn effective_security_prefers_method_then_interface_and_supports_explicit_none() {
    let interface = vec![bare("http_basic")];
    let method = vec![builtin("api_key", r#"in="header", name="X-Token""#)];
    assert_eq!(
        effective_security(&interface, &method).expect("method security"),
        Some(vec![HttpSecurityRequirement::ApiKey {
            location: HttpApiKeyLocation::Header,
            name: "X-Token".to_string(),
        }])
    );

    assert_eq!(
        effective_security(&interface, &[bare("no_security")]).expect("no security"),
        Some(Vec::new())
    );

    let inherited = effective_security_with_origin(&interface, &[]).expect("inherited");
    assert_eq!(
        inherited,
        Some(HttpSecurityProfile {
            origin: HttpSecurityOrigin::Interface,
            requirements: vec![HttpSecurityRequirement::HttpBasic],
        })
    );
}

#[test]
fn collect_security_rejects_duplicates_and_conflicts() {
    let duplicate = collect_security(&[bare("http_basic"), bare("http_basic")])
        .err()
        .expect("duplicate basic");
    assert!(duplicate.contains("duplicate @http_basic"));

    let conflict = collect_security(&[bare("no_security"), bare("http_bearer")])
        .err()
        .expect("conflict");
    assert!(conflict.contains("@no_security cannot be combined"));
}

#[test]
fn parse_api_key_validates_location_and_name() {
    assert_eq!(
        parse_api_key(&builtin("api_key", r#"in="query", name="token""#)).expect("api key"),
        HttpSecurityRequirement::ApiKey {
            location: HttpApiKeyLocation::Query,
            name: "token".to_string(),
        }
    );
    assert!(
        parse_api_key(&builtin("api_key", r#"in="matrix", name="token""#))
            .expect_err("invalid location")
            .contains("header|query|cookie")
    );
    assert!(
        parse_api_key(&builtin("api_key", r#"in="header""#))
            .expect_err("missing name")
            .contains("non-empty name")
    );
    assert!(
        parse_api_key(&bare("api_key"))
            .expect_err("missing in")
            .contains("requires in=... and name=...")
    );
}
