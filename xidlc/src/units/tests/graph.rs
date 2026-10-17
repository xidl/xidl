use super::support::TempDir;
use crate::units::{ImportError, UnitGraph};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Builds a graph over `inputs` without dependency mounts.
fn build(inputs: &[PathBuf]) -> Result<UnitGraph, ImportError> {
    UnitGraph::build(inputs, &BTreeMap::new())
}

/// The prefixes of `graph`, in dependency order.
fn prefixes(graph: &UnitGraph) -> Vec<String> {
    graph
        .units()
        .iter()
        .map(|unit| unit.prefix().to_string())
        .collect()
}

#[test]
fn a_directory_import_resolves_to_index_idl() {
    let dir = TempDir::new("graph-directory");
    dir.write("timestamp/index.idl", "struct Instant { string at; };\n");
    let app = dir.write("app.idl", "import \"timestamp\";\n");

    let graph = build(std::slice::from_ref(&app)).expect("graph builds");
    assert_eq!(prefixes(&graph), vec!["timestamp", "app"]);

    let app_unit = &graph.units()[1];
    assert_eq!(app_unit.prefix(), "app");
    assert_eq!(app_unit.file(), app.as_path());
    assert_eq!(app_unit.generation_input(), app);

    let import = &app_unit.imports()[0];
    assert_eq!(import.prefix(), "timestamp");
    assert_eq!(import.version(), None);
    assert_eq!(import.file(), dir.path().join("timestamp/index.idl"));
}

#[test]
fn a_directory_unit_is_generated_under_its_prefix() {
    let dir = TempDir::new("graph-entry-name");
    dir.write("timestamp/index.idl", "struct Instant { string at; };\n");
    let app = dir.write("app.idl", "import \"timestamp\";\n");

    let graph = build(&[app]).expect("graph builds");
    let entry = &graph.units()[0];
    assert_eq!(entry.prefix(), "timestamp");
    assert_eq!(
        entry.generation_input(),
        dir.path().join("timestamp/timestamp.idl")
    );
}

#[test]
fn a_named_file_import_uses_its_file_stem() {
    let dir = TempDir::new("graph-named-file");
    dir.write("pkg/util.idl", "struct Id { string value; };\n");
    let app = dir.write(
        "app.idl",
        "import \"pkg/util.idl@v1\";\n\nstruct Event { util::Id id; };\n",
    );

    let graph = build(&[app]).expect("graph builds");
    assert_eq!(prefixes(&graph), vec!["util", "app"]);
    let import = &graph.units()[1].imports()[0];
    assert_eq!(import.prefix(), "util");
    assert_eq!(import.version(), Some("v1"));
    assert_eq!(import.path(), "pkg/util.idl@v1");
}

#[test]
fn imports_are_followed_transitively_in_dependency_order() {
    let dir = TempDir::new("graph-recursive");
    dir.write("c.idl", "struct C { string value; };\n");
    dir.write(
        "b.idl",
        "import \"c.idl\";\n\nstruct B { string value; };\n",
    );
    let a = dir.write(
        "a.idl",
        "import \"b.idl\";\n\nstruct A { string value; };\n",
    );

    let graph = build(&[a]).expect("graph builds");
    assert_eq!(prefixes(&graph), vec!["c", "b", "a"]);
}

#[test]
fn a_shared_unit_is_registered_once() {
    let dir = TempDir::new("graph-diamond");
    dir.write("d.idl", "struct D { string value; };\n");
    dir.write("b.idl", "import \"d.idl\";\n");
    dir.write("c.idl", "import \"d.idl\";\n");
    let a = dir.write("a.idl", "import \"b.idl\";\nimport \"c.idl\";\n");

    let graph = build(&[a]).expect("graph builds");
    assert_eq!(prefixes(&graph), vec!["d", "b", "c", "a"]);
}

#[test]
fn an_import_cycle_is_reported_with_its_chain() {
    let dir = TempDir::new("graph-cycle");
    let a = dir.write("a.idl", "import \"b.idl\";\n");
    dir.write("b.idl", "import \"a.idl\";\n");

    let err = build(&[a]).expect_err("cycle is rejected");
    let ImportError::Cycle { chain } = err else {
        panic!("expected a cycle error, got {err}");
    };
    assert_eq!(chain.len(), 3, "chain: {chain:?}");
    assert!(chain[0].ends_with("a.idl"));
    assert!(chain[2].ends_with("a.idl"));
}

#[test]
fn a_missing_import_target_names_the_importer() {
    let dir = TempDir::new("graph-missing");
    let app = dir.write("app.idl", "import \"nope.idl\";\n");

    let err = build(&[app]).expect_err("missing target is rejected");
    let ImportError::Unresolved { import, reason, .. } = err else {
        panic!("expected an unresolved import, got {err}");
    };
    assert_eq!(import, "nope.idl");
    assert!(reason.contains("nope.idl"), "reason: {reason}");
}

#[test]
fn a_directory_without_index_idl_is_rejected() {
    let dir = TempDir::new("graph-no-index");
    dir.write("pkg/other.idl", "struct Other { string value; };\n");
    let app = dir.write("app.idl", "import \"pkg\";\n");

    let err = build(&[app]).expect_err("directory import needs an entry");
    let ImportError::Unresolved { reason, .. } = err else {
        panic!("expected an unresolved import, got {err}");
    };
    assert!(reason.contains("index.idl"), "reason: {reason}");
}

#[test]
fn two_units_cannot_claim_one_prefix() {
    let dir = TempDir::new("graph-prefix-conflict");
    dir.write("a/util.idl", "struct A { string value; };\n");
    dir.write("b/util.idl", "struct B { string value; };\n");
    let app = dir.write(
        "app.idl",
        "import \"a/util.idl\";\nimport \"b/util.idl\";\n",
    );

    let err = build(&[app]).expect_err("prefix conflict is rejected");
    let ImportError::PrefixConflict { prefix, .. } = err else {
        panic!("expected a prefix conflict, got {err}");
    };
    assert_eq!(prefix, "util");
}

#[test]
fn one_file_cannot_be_imported_under_two_prefixes() {
    let dir = TempDir::new("graph-aliased-unit");
    dir.write("pkg/index.idl", "struct Id { string value; };\n");
    let app = dir.write("app.idl", "import \"pkg\";\nimport \"pkg/index.idl\";\n");

    let err = build(&[app]).expect_err("unit alias is rejected");
    let ImportError::UnitAliased { first, second, .. } = err else {
        panic!("expected a unit alias error, got {err}");
    };
    assert_eq!(first, "pkg");
    assert_eq!(second, "index");
}

#[test]
fn an_unquoted_import_is_rejected() {
    let dir = TempDir::new("graph-unquoted");
    let app = dir.write("app.idl", "import pkg;\n");

    let err = build(&[app]).expect_err("unquoted import is rejected");
    let ImportError::Unquoted { scope, .. } = err else {
        panic!("expected an unquoted import error, got {err}");
    };
    assert_eq!(scope, "pkg");
}

#[test]
fn imports_resolve_inside_a_dependency_mount() {
    let dir = TempDir::new("graph-mount");
    let dep = dir.write(
        "cache/timestamp/index.idl",
        "struct Instant { string at; };\n",
    );
    let app = dir.write("app.idl", "import \"timestamp\";\n");
    let mounts = BTreeMap::from([(
        "timestamp".to_string(),
        dep.parent().expect("dep dir").to_path_buf(),
    )]);

    let graph = UnitGraph::build(&[app], &mounts).expect("graph builds");
    assert_eq!(prefixes(&graph), vec!["timestamp", "app"]);
    assert_eq!(graph.units()[0].file(), dep.as_path());
}

#[test]
fn a_unit_import_property_carries_its_prefix_and_version() {
    let dir = TempDir::new("graph-property");
    let dep = dir.write("pkg/util.idl", "struct Id { string value; };\n");
    let app = dir.write("app.idl", "import \"pkg/util.idl@v1\";\n");

    let graph = build(&[app]).expect("graph builds");
    let property = graph.units()[1].import_property();
    let imports = property.as_array().expect("array");
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0]["prefix"], "util");
    assert_eq!(imports[0]["version"], "v1");
    assert_eq!(imports[0]["file"], dep.to_string_lossy().as_ref());
}
