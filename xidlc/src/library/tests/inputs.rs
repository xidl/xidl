//! Input pattern tests.

use super::support::*;
use crate::library::*;
#[test]
fn library_inputs_default_to_idl_files_beside_the_manifest() {
    let dir = temp_dir("inputs-default");
    write(&dir.join("lib.idl"), "struct A { int32 v; };\n");
    write(&dir.join("other.idl"), "struct B { int32 v; };\n");
    write(&dir.join("notes.txt"), "not idl\n");
    write(&dir.join("nested/deep.idl"), "struct C { int32 v; };\n");

    let manifest = manifest_for(&dir, "name = \"a\"\nversion = \"1.0.0\"\n");
    let inputs = LibraryInputs::of(&manifest, &dir).resolve().unwrap();

    let names = inputs
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(names, ["lib.idl", "other.idl"]);
    cleanup(dir);
}

#[test]
fn library_inputs_honour_files_and_exclude() {
    let dir = temp_dir("inputs-patterns");
    write(&dir.join("a.idl"), "struct A { int32 v; };\n");
    write(&dir.join("draft.idl"), "struct D { int32 v; };\n");
    write(&dir.join("lib/b.idl"), "struct B { int32 v; };\n");

    let manifest = manifest_for(
        &dir,
        "name = \"a\"\nversion = \"1.0.0\"\nfiles = [\"*.idl\", \"lib/*.idl\"]\nexclude = [\"draft.idl\"]\n",
    );
    let inputs = LibraryInputs::of(&manifest, &dir).resolve().unwrap();

    let names = inputs
        .iter()
        .map(|path| {
            path.strip_prefix(&dir)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<Vec<_>>();
    assert_eq!(names, ["a.idl", "lib/b.idl"]);
    cleanup(dir);
}

#[test]
fn library_inputs_fail_when_nothing_matches() {
    let dir = temp_dir("inputs-empty");
    let manifest = manifest_for(&dir, "name = \"a\"\nversion = \"1.0.0\"\n");

    let err = LibraryInputs::of(&manifest, &dir).resolve().unwrap_err();
    assert!(matches!(err, LibraryError::NoInputs { .. }), "{err}");
    cleanup(dir);
}
