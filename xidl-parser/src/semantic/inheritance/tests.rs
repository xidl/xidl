use crate::hir::{ScopedName, Specification, TypeSpec};

fn parse(source: &str) -> Specification {
    let typed = crate::parser::parser_text(source).expect("valid inheritance IDL");
    Specification::from_typed_ast_with_path(typed, "inheritance_scope.idl")
        .expect("valid inheritance semantics")
}

fn named(path: &[&str]) -> TypeSpec {
    TypeSpec::ScopedName(ScopedName {
        is_root: true,
        name: path.iter().map(|part| part.to_string()).collect(),
    })
}

#[test]
fn ancestor_references_bind_before_derived_scope_can_shadow_them() {
    let mut spec = parse(
        r#"
        typedef string RootId;
        module geo {
            typedef string Identifier;
            enum Country { US, CA };
            struct Point { long latitude; };
            module source {
                struct Identity {
                    Identifier id;
                    Country country;
                    sequence<map<Identifier, sequence<Point>>> points;
                    ::RootId root_id;
                };
            };
        };
        module shipping {
            typedef long Identifier;
            typedef long Country;
            typedef long Point;
            typedef long RootId;
            struct Postal : geo::source::Identity {};
            struct Address : Postal {};
        };
        "#,
    );
    let structs = super::collect_structs(&spec.0);
    let inherited = &structs["shipping::Address"].inherited;
    assert_eq!(
        serde_json::to_value(&inherited[0].ty).unwrap(),
        serde_json::to_value(named(&["geo", "Identifier"])).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&inherited[1].ty).unwrap(),
        serde_json::to_value(named(&["geo", "Country"])).unwrap()
    );
    let TypeSpec::SequenceType(sequence) = &inherited[2].ty else {
        panic!("points sequence")
    };
    let TypeSpec::MapType(map) = &*sequence.ty else {
        panic!("points map")
    };
    assert_eq!(
        serde_json::to_value(&map.key).unwrap(),
        serde_json::to_value(named(&["geo", "Identifier"])).unwrap()
    );
    let TypeSpec::SequenceType(values) = &*map.value else {
        panic!("point values")
    };
    assert_eq!(
        serde_json::to_value(&values.ty).unwrap(),
        serde_json::to_value(named(&["geo", "Point"])).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&inherited[3].ty).unwrap(),
        serde_json::to_value(named(&["RootId"])).unwrap()
    );
    let before = serde_json::to_value(&spec).expect("initial semantic model");
    super::analyze(&mut spec).expect("repeat semantic analysis");
    assert_eq!(serde_json::to_value(&spec).unwrap(), before);
}

#[test]
fn interface_local_ancestors_keep_their_declared_type_identity() {
    let spec = parse(
        r#"
        interface Source {
            typedef string Identifier;
            struct Identity { Identifier id; };
        };
        module shipping {
            typedef long Identifier;
            struct Address : Source::Identity {};
        };
        "#,
    );
    let structs = super::collect_structs(&spec.0);
    let inherited = &structs["shipping::Address"].inherited[0].ty;
    assert_eq!(
        serde_json::to_value(inherited).unwrap(),
        serde_json::to_value(named(&["Source", "Identifier"])).unwrap()
    );
}

#[test]
fn inherited_collection_bounds_resolve_while_other_expressions_keep_their_declared_constants() {
    let spec = parse(
        r#"
        module limits {
            const long WIDTH = 8;
            const long SCALE = 2;
            struct Receipt {
                long digits[WIDTH];
                sequence<string, WIDTH> lines;
                map<string, long, WIDTH> totals;
                string<WIDTH> reference;
                wstring<WIDTH> description;
                fixed<WIDTH, SCALE> amount;
                long precision default +WIDTH - SCALE;
                long retry_count default 1;
            };
        };
        module shipping {
            const long WIDTH = 1;
            const long SCALE = 0;
            struct Receipt : limits::Receipt {};
        };
        "#,
    );
    let structs = super::collect_structs(&spec.0);
    let inherited = &structs["shipping::Receipt"].inherited;
    let crate::hir::Declarator::ArrayDeclarator(array) = &inherited[0].ident[0] else {
        panic!("inherited digits array");
    };
    assert_eq!(crate::hir::const_expr_to_i64(&array.len[0].0), Some(8));
    let crate::hir::TypeSpec::SequenceType(sequence) = &inherited[1].ty else {
        panic!("inherited lines sequence");
    };
    assert_eq!(
        crate::hir::const_expr_to_i64(&sequence.len.as_ref().unwrap().0),
        Some(8)
    );
    let json = serde_json::to_value(inherited).expect("inherited receipt fields");
    let mut names = Vec::new();
    collect_names(&json, &mut names);
    let counts = names
        .into_iter()
        .fold(std::collections::BTreeMap::new(), |mut counts, name| {
            *counts.entry(name).or_insert(0) += 1;
            counts
        });
    assert_eq!(
        counts,
        std::collections::BTreeMap::from([
            ("limits::WIDTH".to_string(), 5),
            ("limits::SCALE".to_string(), 2),
        ])
    );
}

fn collect_names(value: &serde_json::Value, names: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(path) = object.get("name").and_then(serde_json::Value::as_array) {
                assert_eq!(object["is_root"], true);
                names.push(
                    path.iter()
                        .map(|part| part.as_str().expect("name segment"))
                        .collect::<Vec<_>>()
                        .join("::"),
                );
            }
            for value in object.values() {
                collect_names(value, names);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_names(value, names);
            }
        }
        _ => {}
    }
}
