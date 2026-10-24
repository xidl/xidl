use super::parse;
use crate::hir::Specification;

fn union_named<'a>(spec: &'a Specification, name: &str) -> &'a crate::hir::UnionDef {
    fn find<'a>(
        defs: &'a [crate::hir::Definition],
        name: &str,
    ) -> Option<&'a crate::hir::UnionDef> {
        for def in defs {
            match def {
                crate::hir::Definition::ConstrTypeDcl(crate::hir::ConstrTypeDcl::UnionDef(u))
                | crate::hir::Definition::TypeDcl(crate::hir::TypeDcl::ConstrTypeDcl(
                    crate::hir::ConstrTypeDcl::UnionDef(u),
                )) if u.ident == name => {
                    return Some(u);
                }
                crate::hir::Definition::ModuleDcl(module) => {
                    if let Some(found) = find(&module.definition, name) {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }
        None
    }
    find(&spec.0, name).expect("union declaration")
}

fn scoped(name: &str) -> crate::hir::ScopedName {
    let is_root = name.starts_with("::");
    crate::hir::ScopedName {
        is_root,
        name: name
            .trim_start_matches("::")
            .split("::")
            .map(str::to_string)
            .collect(),
    }
}

fn path(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

#[test]
fn union_switch_on_declared_enum_records_its_enumerators() {
    let spec = parse(
        r#"
        enum Tag { A, B };
        union Simple switch (Tag) {
            case A: int32 a;
            case B: string b;
        };
        "#,
    );
    let switch_enum = union_named(&spec, "Simple")
        .switch_enum
        .as_ref()
        .expect("switch enum resolves");
    assert_eq!(switch_enum.path, path(&["Tag"]));
    assert_eq!(switch_enum.enumerators, path(&["A", "B"]));
}

#[test]
fn union_switch_on_primitive_or_unknown_type_resolves_nothing() {
    let spec = parse(
        r#"
        union Numbered switch (int32) {
            case 1: string one;
        };
        union Flagged switch (boolean) {
            case TRUE: string yes;
        };
        union External switch (Missing) {
            case Missing::X: string x;
        };
        "#,
    );
    assert!(union_named(&spec, "Numbered").switch_enum.is_none());
    assert!(union_named(&spec, "Flagged").switch_enum.is_none());
    assert!(union_named(&spec, "External").switch_enum.is_none());
}

#[test]
fn union_switch_resolves_across_modules_and_tells_enumerators_from_constants() {
    let spec = parse(
        r#"
        module choices {
            enum Kind { Text, Number };
            const Kind TEXT = Kind::Text;
        };
        module left {
            union Choice switch (::choices::Kind) {
                case Text: string text;
                case ::choices::Kind::Number: int64 number;
                case ::choices::TEXT: boolean flagged;
            };
        };
        "#,
    );
    let switch_enum = union_named(&spec, "Choice")
        .switch_enum
        .as_ref()
        .expect("cross-module switch enum resolves");
    assert_eq!(switch_enum.path, path(&["choices", "Kind"]));
    assert_eq!(switch_enum.enumerator_named(&scoped("Text")), Some("Text"));
    assert_eq!(
        switch_enum.enumerator_named(&scoped("::choices::Kind::Number")),
        Some("Number")
    );
    assert_eq!(
        switch_enum.enumerator_named(&scoped("Kind::Number")),
        Some("Number")
    );
    assert_eq!(
        switch_enum.enumerator_named(&scoped("::choices::TEXT")),
        None
    );
    assert_eq!(switch_enum.enumerator_named(&scoped("TEXT")), None);
    assert_eq!(switch_enum.enumerator_named(&scoped("other::Text")), None);
}

#[test]
fn union_switch_follows_typedef_chains_in_their_declaring_scope() {
    let spec = parse(
        r#"
        module shapes {
            enum Kind { Round, Square };
            typedef Kind Discriminant;
        };
        module api {
            typedef shapes::Discriminant Selector;
            union Shape switch (Selector) {
                case Round: double radius;
                case Square: double side;
            };
        };
        "#,
    );
    let switch_enum = union_named(&spec, "Shape")
        .switch_enum
        .as_ref()
        .expect("typedef chain resolves to the enum");
    assert_eq!(switch_enum.path, path(&["shapes", "Kind"]));
    assert_eq!(switch_enum.enumerators, path(&["Round", "Square"]));
}

#[test]
fn union_switch_on_a_typedef_cycle_resolves_nothing() {
    let spec = parse(
        r#"
        typedef Second First;
        typedef First Second;
        union Looping switch (First) {
            case 1: string one;
        };
        "#,
    );
    assert!(union_named(&spec, "Looping").switch_enum.is_none());
}
