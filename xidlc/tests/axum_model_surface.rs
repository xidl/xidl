#![cfg(all(feature = "gen-rust-axum", feature = "gen-rust-axum-adapters"))]

use std::{collections::HashMap, path::Path};

fn generate(target: &str, source: &str) -> String {
    let files = xidlc::driver::Generator::new(target.into())
        .generate_from_idl(source, Path::new("models.idl"), HashMap::new())
        .expect("valid model contract");
    files[0].content().to_string()
}

#[test]
fn retired_alias_remains_available_to_adapters_in_head() {
    let source = r#"
@version enum Versions { v1, v2, v3 };
typedef string Reference;
struct Address {
    string id;
    @removed(Versions::v3) Reference reference;
};
struct Change {
    @optional @removed(Versions::v3) Reference reference;
};
interface Addresses {
    @get(path="/address") Address read();
    @patch(path="/address") Address update(@body Change value);
};
"#;
    let models = generate("rust-axum", source);
    let adapters = generate("rust-axum-adapters", source);
    for version in ["v1", "v2", "v3"] {
        let module = models
            .split(&format!("pub mod {version} {{"))
            .nth(1)
            .expect("supported version module")
            .split("\npub mod ")
            .next()
            .expect("module body");
        assert!(
            module.contains("pub type Reference = String;"),
            "{version} must retain the declared alias"
        );
    }
    let head = models.split("pub mod v3 {").nth(1).expect("Head models");
    assert!(head.contains("pub type Reference = String;"));
    assert!(
        !head.contains("pub reference:"),
        "retired fields stay retired"
    );
    assert!(adapters.contains("pub reference: Option<v3::Reference>"));
    assert!(adapters.contains("V3(v3::Reference)"));
}

#[test]
fn unreferenced_declared_models_keep_their_dependency_graph() {
    let output = generate(
        "rust-axum",
        r#"
module catalog {
    enum State { pending, complete };
    struct Record { State state; };
    typedef sequence<Record> Records;
};
struct Archive { catalog::Records records; };
interface Health {
    @get(path="/health") boolean read();
};
native Handle;
"#,
    );
    for declaration in [
        "pub enum State",
        "pub struct Record",
        "pub type Records = Vec<Record>",
        "pub struct Archive",
        "pub records: catalog::Records",
        "pub type Handle = *mut ::std::ffi::c_void;",
    ] {
        assert!(output.contains(declaration), "missing model: {declaration}");
    }
    assert!(!output.contains("pub mod xidl"), "builtins stay external");
}

#[test]
fn selected_models_obey_declaration_lifecycles() {
    let source = r#"
@version enum Versions { v1, v2 };
typedef string Reference;
@removed(Versions::v2) struct Legacy { Reference value; };
@added(Versions::v2) struct Current { Reference value; };
interface Health { @get(path="/health") boolean read(); };
"#;
    for (version, present, absent) in [("v1", "Legacy", "Current"), ("v2", "Current", "Legacy")] {
        let files = xidlc::driver::Generator::new("rust-axum".into())
            .generate_from_idl(
                source,
                Path::new("models.idl"),
                HashMap::from([("api_version".into(), version.into())]),
            )
            .expect("selected model contract");
        assert_eq!(files.len(), 1);
        let output = files[0].content();
        assert!(output.contains("pub type Reference = String;"));
        assert!(output.contains(&format!("pub struct {present}")));
        assert!(!output.contains(&format!("pub struct {absent}")));
    }
}
