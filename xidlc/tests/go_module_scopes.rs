#![cfg(all(feature = "gen-go", feature = "gen-go-rest"))]

use std::{collections::BTreeMap, collections::HashMap, fs, path::Path};

use xidlc::driver::{ArgsGenerate, Driver, File, Generator};

fn generated(
    target: &str,
    source: &str,
    properties: HashMap<String, serde_json::Value>,
) -> Vec<File> {
    Generator::new(target.to_string())
        .generate_from_idl(source, Path::new("scopes.idl"), properties)
        .expect("generate scoped Go contract")
}

fn files_by_name(files: Vec<File>) -> BTreeMap<String, String> {
    files
        .into_iter()
        .map(|file| (file.path().to_string(), file.content().to_string()))
        .collect()
}

#[test]
fn split_modules_snapshot_the_names_used_by_go_and_go_rest() {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../bdd/features/data/go_scopes/api.idl");
    for target in ["go", "go-rest"] {
        let output = tempfile::tempdir().expect("Go scope output directory");
        Driver::run(ArgsGenerate {
            lang: target.to_string(),
            out_dir: output.path().to_string_lossy().into_owned(),
            client: true,
            server: true,
            dry_run: false,
            mock: false,
            api_version: None,
            api_versions: None,
            openapi_version: None,
            files: vec![source.clone()],
        })
        .expect("generate included and imported Go definitions");
        let mut files = BTreeMap::new();
        for name in ["api.go", "api_http.go", "sources/origin.go"] {
            let path = output.path().join(name);
            if path.exists() {
                files.insert(name, fs::read_to_string(path).expect("generated Go source"));
            }
        }
        assert!(
            !output.path().join("sources/origin_http.go").exists(),
            "a model-only import must not emit an empty HTTP file with unused imports"
        );
        let model = &files["api.go"];
        assert!(model.contains("Detail LedgerDetail"));
        assert!(model.contains("Root Detail"));
        assert!(model.contains("Origin sources.HttpApiDetail"));
        assert!(model.contains("InitialState sources.HTTPState = sources.HTTPStateReady"));
        assert!(model.contains("type LedgerEntriesView = LedgerDetail"));
        insta::assert_json_snapshot!(format!("split_module_{target}"), files);
    }
}

#[test]
fn interface_exports_survive_disabling_interface_methods() {
    let source = r#"
        module app {
            interface Api {
                typedef string Value;
                struct Detail { Value message; };
                exception Failure { Detail detail; };
                attribute Detail current;
                Detail read(in Detail value) raises(Failure);
            };
            struct Row { Api::Detail detail; };
        };
    "#;
    for enabled in [false, true] {
        let files = files_by_name(generated(
            "go",
            source,
            HashMap::from([("enable_interfaces".into(), enabled.into())]),
        ));
        let model = &files["scopes.go"];
        assert!(model.contains("type AppApiValue = string"));
        assert!(model.contains("Message AppApiValue"));
        assert!(model.contains("Detail AppApiDetail"));
        assert_eq!(model.contains("type AppApi interface"), enabled);
        insta::assert_json_snapshot!(format!("interface_exports_{enabled}"), files);
    }
}

#[test]
fn interface_types_keep_their_module_name() {
    let source = r#"
        module app {
            interface Api { void ping(); };
            struct Endpoint { Api service; };
        };
    "#;
    let files = files_by_name(generated("go", source, HashMap::new()));
    assert!(files["scopes.go"].contains("Service AppApi"));
}

#[test]
fn projected_versions_use_the_same_lexical_type_lookup() {
    let source = r#"
        @version enum V { v1, v2 };
        module outer {
            struct Detail { string message; };
            module inner {
                struct Entry {
                    Detail detail;
                    @added(V::v2) @optional string note;
                };
                @no_security interface Store {
                    @post(path="/entry") Entry save(@body Entry value);
                };
            };
        };
    "#;
    let files = files_by_name(generated(
        "go-rest",
        source,
        HashMap::from([
            ("enable_client".into(), true.into()),
            ("enable_server".into(), true.into()),
        ]),
    ));
    let model = &files["scopes.go"];
    for version in ["V1", "V2"] {
        assert!(model.contains(&format!("Detail {version}OuterDetail")));
        assert!(files["scopes_http.go"].contains(&format!("{version}OuterInnerEntry")));
    }
    insta::assert_json_snapshot!("projected_module_scopes", files);
}

#[test]
fn a_root_exception_is_not_rebound_to_an_import_prefix() {
    let root = tempfile::tempdir().expect("exception scope fixture");
    fs::create_dir(root.path().join("pkg")).expect("import directory");
    fs::write(
        root.path().join("pkg/left.idl"),
        "struct Other { string id; };",
    )
    .expect("imported model");
    let source = root.path().join("api.idl");
    fs::write(
        &source,
        r#"
        import "pkg/left.idl";
        module left {
            struct Detail { string message; };
            @http(409) exception Failure { Detail detail; };
        };
        @no_security interface Api {
            @get(path="/detail") string read() raises(::left::Failure);
        };
    "#,
    )
    .expect("local exception contract");
    let output = root.path().join("output");
    Driver::run(ArgsGenerate {
        lang: "go-rest".into(),
        out_dir: output.to_string_lossy().into_owned(),
        client: true,
        server: true,
        dry_run: false,
        mock: false,
        api_version: None,
        api_versions: None,
        openapi_version: None,
        files: vec![source],
    })
    .expect("root reference selects the local exception");
    let http = fs::read_to_string(output.join("api_http.go")).expect("HTTP source");
    assert!(http.contains("*LeftFailure"));
    assert!(!http.contains("pkg.Failure"));
    assert!(
        !http.contains("\"pkg\""),
        "an unused import must stay unused"
    );
}
