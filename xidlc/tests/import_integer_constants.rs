#![cfg(feature = "gen-rust")]

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::process::Command;
use xidlc::driver::{ArgsGenerate, Driver};

struct ConstantsGraph {
    root: tempfile::TempDir,
}

impl ConstantsGraph {
    fn new(sources: &[(&str, &str)]) -> Self {
        let root = tempfile::tempdir().expect("integer unit graph directory");
        for (name, source) in sources {
            fs::write(root.path().join(name), source).expect("integer source unit");
        }
        Self { root }
    }

    fn generate(&self, entries: &[&str]) -> BTreeMap<String, String> {
        let output = self.root.path().join("generated");
        Driver::run_with_props(
            ArgsGenerate {
                lang: "rust".into(),
                out_dir: output.to_string_lossy().into_owned(),
                client: false,
                server: false,
                dry_run: false,
                mock: false,
                api_version: None,
                api_versions: None,
                openapi_version: None,
                files: entries
                    .iter()
                    .map(|name| self.root.path().join(name))
                    .collect(),
            },
            HashMap::from([
                ("xidlc_version".into(), "0.0.1".into()),
                ("xidlc_shorthash".into(), "snapshot".into()),
            ]),
        )
        .expect("generate imported integer contracts");
        fs::read_dir(&output)
            .expect("generated integer units")
            .map(|entry| {
                let path = entry.unwrap().path();
                (
                    path.file_name().unwrap().to_string_lossy().into_owned(),
                    fs::read_to_string(&path).unwrap(),
                )
            })
            .collect()
    }

    fn execute(&self, current: u32) {
        self.execute_source(
            &r#"
            #[path="generated/base.rs"] pub mod base;
            #[path="generated/middle.rs"] pub mod middle;
            #[path="generated/api.rs"] pub mod api;
            #[path="generated/old.rs"] pub mod old;
            #[path="generated/head.rs"] pub mod head;
            fn main() {
                assert_eq!(api::PUBLISHED, 2);
                assert_eq!(api::CURRENT, __CURRENT__);
                assert_eq!(api::ROOTED, 1);
                assert_eq!(api::PROMOTED, 4294967296);
                assert_eq!(api::PRICE, 1.5);
                assert_eq!(old::PUBLISHED, 2);
                assert_eq!(head::CURRENT, __CURRENT__);
                let old: middle::OldSlots = [0; 2];
                let new: middle::NewSlots = [0; __CURRENT__];
                assert_eq!(old.len() + new.len(), 2 + __CURRENT__);
            }
        "#
            .replace("__CURRENT__", &current.to_string()),
        );
    }

    fn execute_source(&self, source: &str) {
        let output = self.root.path();
        fs::write(output.join("main.rs"), source).unwrap();
        let binary = output.join(format!("constants{}", std::env::consts::EXE_SUFFIX));
        let compiled = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
            .arg("--edition=2024")
            .arg(output.join("main.rs"))
            .arg("-o")
            .arg(&binary)
            .output()
            .expect("compile the generated constant unit graph");
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        let executed = Command::new(binary)
            .output()
            .expect("execute integer unit graph");
        assert!(
            executed.status.success(),
            "{}",
            String::from_utf8_lossy(&executed.stderr)
        );
    }
}

const BASE: &str = r#"
    @version enum V { v1, v2 };
    const unsigned long MASK = 0xfffffffe;
    const unsigned long MAXIMUM = 0xffffffff;
    @removed(V::v2) module policy {
        typedef unsigned long Word;
        const Word MASK = 0xfffffffd;
        const Word WIDTH = ~MASK;
        const Word ROOTED = ~::MASK;
    };
    @added(V::v2) module policy {
        typedef unsigned long Word;
        const Word MASK = 0xfffffffc;
        const Word WIDTH = ~MASK;
        const Word ROOTED = ~::MASK;
    };
"#;

#[test]
fn transitive_versioned_constants_compile_and_keep_published_consumers_frozen() {
    let graph = ConstantsGraph::new(&[
        ("base.idl", BASE),
        (
            "middle.idl",
            r#"
            import "base.idl";
            typedef base::v1::policy::Word Word;
            const Word OLD = base::v1::policy::WIDTH;
            const unsigned long NEW = base::v2::policy::WIDTH;
            const unsigned long ROOTED = base::v1::policy::ROOTED;
            const unsigned long long PROMOTED = base::v1::MAXIMUM + 1;
            typedef long OldSlots[OLD];
            typedef long NewSlots[NEW];
        "#,
        ),
        (
            "api.idl",
            r#"
            import "middle.idl";
            const unsigned long MASK = 100;
            typedef double Word;
            const Word PRICE = 1.5;
            const unsigned long PUBLISHED = middle::OLD;
            const unsigned long CURRENT = middle::NEW;
            const unsigned long ROOTED = middle::ROOTED;
            const unsigned long long PROMOTED = middle::PROMOTED;
        "#,
        ),
        (
            "old.idl",
            r#"
            import "base.idl@v1";
            const unsigned long PUBLISHED = base::v1::policy::WIDTH;
        "#,
        ),
        (
            "head.idl",
            r#"
            import "base.idl@v2";
            const unsigned long CURRENT = base::v2::policy::WIDTH;
        "#,
        ),
    ]);
    let first = graph.generate(&["api.idl", "old.idl", "head.idl"]);
    assert_eq!(first.len(), 5);
    insta::assert_debug_snapshot!("versioned_integer_units", first);
    graph.execute(3);

    fs::write(
        graph.root.path().join("base.idl"),
        BASE.replace("0xfffffffc", "0xfffffffb"),
    )
    .expect("publish the changed head policy");
    let changed = graph.generate(&["api.idl", "old.idl", "head.idl"]);
    assert_eq!(first["old.rs"], changed["old.rs"]);
    assert_ne!(first["head.rs"], changed["head.rs"]);
    graph.execute(4);
    assert!(changed["middle.rs"].contains("pub type NewSlots = [i32; 4];"));
}

#[test]
fn local_included_and_imported_constants_share_idl_width_rules() {
    let dependency = "const unsigned long MASK = 0xffffffff;";
    for source in [
        format!("{dependency} const unsigned long long REST = MASK + 1;"),
        "#include \"dep.idl\"\nconst unsigned long long REST = MASK + 1;".into(),
        "import \"dep.idl\"; const unsigned long long REST = dep::MASK + 1;".into(),
    ] {
        let graph = ConstantsGraph::new(&[("dep.idl", dependency), ("api.idl", &source)]);
        let files = graph.generate(&["api.idl"]);
        assert!(files["api.rs"].contains("pub const REST: u64 = {"));
        assert!(files["api.rs"].contains(" + "));
        graph.execute_source(
            r#"
            #[path="generated/api.rs"] mod api;
            fn main() {
                let value = api::REST;
                let _: &u64 = &value;
                assert_eq!(value, 4294967296);
            }
        "#,
        );
    }
}
