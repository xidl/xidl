use crate::units::ImportPath;

#[test]
fn splits_a_trailing_version_selector() {
    let path = ImportPath::parse("pkg/util.idl@v1");
    assert_eq!(path.target(), "pkg/util.idl");
    assert_eq!(path.version(), Some("v1"));
    assert_eq!(path.prefix().as_deref(), Some("util"));
}

#[test]
fn keeps_a_leading_at_sign_in_the_path() {
    let path = ImportPath::parse("@xidl/http");
    assert_eq!(path.target(), "@xidl/http");
    assert_eq!(path.version(), None);
    assert_eq!(path.prefix().as_deref(), Some("http"));
}

#[test]
fn treats_an_empty_selector_as_no_version() {
    let path = ImportPath::parse("pkg/util.idl@");
    assert_eq!(path.target(), "pkg/util.idl@");
    assert_eq!(path.version(), None);
    assert_eq!(path.prefix().as_deref(), Some("util.idl@"));
}

#[test]
fn takes_the_directory_name_of_a_directory_import() {
    let path = ImportPath::parse("pkg/");
    assert_eq!(path.prefix().as_deref(), Some("pkg"));
}

#[test]
fn rejects_paths_without_a_usable_name() {
    for raw in ["", "/", "..", "./"] {
        assert_eq!(ImportPath::parse(raw).prefix(), None, "raw: {raw:?}");
    }
}
