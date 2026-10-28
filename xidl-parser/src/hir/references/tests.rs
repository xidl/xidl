use crate::hir::{self, VisitScopedNames};

#[test]
fn version_namespace_removal_visits_bounds_without_rewriting_dependency_names() {
    let typed = crate::parser::parser_text(
        r#"
        #pragma xidlc package billing
        module dependency { typedef long Count; };
        module v1 {
            const long Limit = 2;
            const ::dependency::Count Maximum = ::v1::Limit;
            const string<::v1::Limit> Unit = "ok";
            const wstring<::v1::Limit> Label = L"ok";
            typedef long Item;
            typedef struct Inline { long values[::v1::Limit]; } Batch;
            struct Base { string id; };
            struct Envelope : ::v1::Base {
                sequence<::v1::Item, ::v1::Limit> items;
                map<string, ::dependency::Count, ::v1::Limit> counts;
                string<::v1::Limit> label;
                wstring<::v1::Limit> wide_label;
                fixed<::v1::Limit, 0> amount;
                long grid[::v1::Limit];
            };
            exception Failure { ::v1::Item code; };
            interface Parent {};
            interface Ledger : ::v1::Parent {
                const ::dependency::Count Size = ::v1::Limit;
                readonly attribute ::v1::Item count raises(::v1::Failure);
                attribute ::v1::Item state getraises(::v1::Failure) setraises(::v1::Failure);
                attribute ::v1::Item enabled setraises(::v1::Failure);
                void apply(in ::v1::Envelope value) raises(::v1::Failure);
            };
            union Change switch (long) {
                case 0: struct Detail { ::v1::Item code; } detail;
                default: ::dependency::Count other;
            };
        };
    "#,
    )
    .expect("valid collection-bound syntax");
    let mut resolver = hir::FsIncludeResolver;
    let mut includes = hir::VersionedIncludes::new(&mut resolver, None);
    let mut definitions = Vec::new();
    // Import binding visits source references before semantic size resolution.
    hir::spec::collect_defs(typed.0, &mut definitions, &mut includes).unwrap();
    let mut names = Vec::new();
    definitions.visit_scoped_names(&mut |name| names.push(name.clone()));
    assert!(
        names
            .iter()
            .filter(|name| name.name == ["v1", "Limit"])
            .count()
            >= 10
    );
    assert!(names.iter().any(|name| name.name == ["v1", "Parent"]));
    assert_eq!(
        names
            .iter()
            .filter(|name| name.name == ["v1", "Failure"])
            .count(),
        5
    );
    assert!(
        names
            .iter()
            .any(|name| name.name == ["dependency", "Count"])
    );
    definitions.remove_root_namespace("v1");
    let mut actual = Vec::new();
    definitions.visit_scoped_names(&mut |name| actual.push(name.clone()));
    let names = actual
        .iter()
        .map(|name| name.name.join("::"))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        names,
        [
            "Base",
            "Envelope",
            "Failure",
            "Item",
            "Limit",
            "Parent",
            "dependency::Count"
        ]
        .map(str::to_owned)
        .into_iter()
        .collect()
    );
    assert!(
        actual.iter().all(|name| name.is_root),
        "absolute references remain absolute"
    );
    let once = serde_json::to_value(&definitions).unwrap();
    definitions.remove_root_namespace("v1");
    assert_eq!(serde_json::to_value(&definitions).unwrap(), once);
    let hir::Definition::ModuleDcl(version) = &definitions[2] else {
        panic!("version module")
    };
    assert_eq!(
        version.ident, "v1",
        "declarations keep their published identity"
    );
}

#[test]
fn standalone_dependency_and_relative_names_keep_their_identity() {
    for (root, path) in [
        (true, vec!["dependency", "Count"]),
        (false, vec!["v1", "Item"]),
    ] {
        let mut name = hir::ScopedName {
            is_root: root,
            name: path.into_iter().map(str::to_owned).collect(),
        };
        let original = name.clone();
        name.remove_root_namespace("v1");
        assert_eq!((name.is_root, name.name), (original.is_root, original.name));
    }
}

#[test]
fn sequence_constant_shape_preserves_element_and_bound_identity() {
    let mut shape = hir::ConstType::SequenceType(hir::SequenceType {
        ty: Box::new(hir::TypeSpec::ScopedName(hir::ScopedName {
            is_root: true,
            name: vec!["models".into(), "Item".into()],
        })),
        len: Some(hir::PositiveIntConst(hir::ConstExpr::ScopedName(
            hir::ScopedName {
                is_root: true,
                name: vec!["v1".into(), "Limit".into()],
            },
        ))),
    });
    shape.remove_root_namespace("v1");
    let mut names = Vec::new();
    shape.visit_scoped_names(&mut |name| names.push(name.name.join("::")));
    assert_eq!(names, ["models::Item", "Limit"]);
    let hir::ConstType::SequenceType(sequence) = &mut shape else {
        panic!("sequence")
    };
    sequence.len = None;
    let mut unbounded = Vec::new();
    shape.visit_scoped_names(&mut |name| unbounded.push(name.name.join("::")));
    assert_eq!(
        unbounded,
        ["models::Item"],
        "an unbounded shape does not gain a size"
    );
}
