use crate::{hir, parser::parser_text, versioning::VersioningRecord};

fn lower(source: &str) -> hir::Specification {
    hir::Specification::from_typed_ast_with_properties_and_path(
        parser_text(source).expect("parse contract"),
        [("expand_interface".into(), false.into())].into(),
        "contract.idl",
    )
    .expect("lower contract")
}

const RETIRED_MODULE: &str = r#"
@version enum Versions { v1, v2 };
@removed(Versions::v2) module Archive {
    #pragma vendor archive
    const long LIMIT = 4;
    native Handle;
    typedef long Matrix[2][3];
    typedef sequence<long, 4> Numbers;
    struct Entry { long id; };
    struct Forward;
    union Future;
    enum State { Ready, Done };
    union Choice switch(long) { case 0: string text; default: long number; };
    bitmask Flags { Read, Write };
    bitset Bits { bitfield<1> enabled; };
    exception Missing { string reason; };
    interface ForwardService;
    interface Empty {};
    interface Service {
        typedef long Identifier;
        const long MAX = 4;
        exception Failure { string reason; };
        attribute string label;
        void read();
    };
};
"#;

#[test]
fn retiring_a_namespace_rejects_references_to_all_its_exported_kinds() {
    for target in [
        "Handle",
        "Matrix",
        "Numbers",
        "Entry",
        "Forward",
        "Future",
        "State",
        "Choice",
        "Flags",
        "Bits",
        "Missing",
        "ForwardService",
        "Empty",
        "Service::Identifier",
        "Service::Failure",
    ] {
        let source = format!("{RETIRED_MODULE}\nstruct Consumer {{ Archive::{target} value; }};");
        let spec = lower(&source);
        let record = VersioningRecord::analyze(&spec).unwrap().unwrap();
        let versions: Vec<_> = record.timeline().iter().map(|(id, _)| id).collect();
        record
            .project(&spec, versions[0])
            .expect("v1 retains archive");
        let error = record
            .project(&spec, versions[1])
            .expect_err("v2 must not publish dangling types");
        let message = error.to_string();
        assert!(message.contains(&format!("Archive::{target}")), "{message}");
        assert!(message.contains("Consumer"), "{message}");
        assert!(message.contains("v2"), "{message}");
    }
    let spec = lower(RETIRED_MODULE);
    let record = VersioningRecord::analyze(&spec).unwrap().unwrap();
    let version = record.timeline().iter().nth(1).unwrap().0;
    let result = record.project(&spec, version).unwrap();
    assert!(!serde_json::to_string(&result).unwrap().contains("Archive"));
}

#[test]
fn projection_preserves_scoped_shapes_bounds_and_prunes_attribute_exceptions() {
    let spec = lower(
        r#"
@version enum Versions { v1, v2 };
const long LIMIT = 4;
struct Item { long id; };
interface Parent {};
interface Forward;
union Pending;
native Token;
typedef struct Inline { long id; } Alias;
@removed(Versions::v2) exception Gone { string reason; };
exception Current { string reason; };
struct Batch {
    sequence<::Item, ::LIMIT> items;
    map<string, ::Item, ::LIMIT> lookup;
    ::Item grid[2][3];
};
interface Store : ::Parent {
    typedef sequence<::Item> Items;
    readonly attribute ::Item item;
    attribute ::Item selected getraises(::Gone, ::Current) setraises(::Gone);
    attribute ::Item updated setraises(::Gone, ::Current);
    ::Item fetch() raises(::Gone, ::Current);
};
"#,
    );
    let record = VersioningRecord::analyze(&spec).unwrap().unwrap();
    for (version, definition) in record.timeline().iter() {
        let projected = record.project(&spec, version).unwrap();
        let json = serde_json::to_string(&projected).unwrap();
        assert!(
            !json.contains("\"is_root\":true"),
            "all moved references must be relative"
        );
        assert!(json.contains("Current"));
        assert_eq!(json.contains("Gone"), definition.ident == "v1");
        let body = projected
            .0
            .iter()
            .find_map(|d| match d {
                hir::Definition::ModuleDcl(module) => Some(&module.definition),
                _ => None,
            })
            .unwrap();
        let store = body
            .iter()
            .find_map(|d| match d {
                hir::Definition::InterfaceDcl(hir::InterfaceDcl {
                    decl: hir::InterfaceDclInner::InterfaceDef(def),
                    ..
                }) if def.header.ident == "Store" => Some(def),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            store.header.parent.as_ref().unwrap().0[0].0.name,
            ["Parent"]
        );
        let names = |values: &[hir::ScopedName]| {
            values
                .iter()
                .map(|name| name.name.join("::"))
                .collect::<Vec<_>>()
        };
        let expected = if definition.ident == "v1" {
            vec!["Gone", "Current"]
        } else {
            vec!["Current"]
        };
        let mut attributes = 0;
        for export in &store.interface_body.as_ref().unwrap().0 {
            let hir::Export::AttrDcl(hir::AttrDcl {
                decl: hir::AttrDclInner::AttrSpec(attribute),
                ..
            }) = export
            else {
                continue;
            };
            let hir::AttrDeclarator::WithRaises { raises, .. } = &attribute.declarator else {
                panic!("raises contract")
            };
            attributes += 1;
            match raises {
                hir::AttrRaisesExpr::Case1(get, Some(set)) => {
                    assert_eq!(names(&get.expr.0), expected);
                    assert_eq!(
                        names(&set.expr.0),
                        if definition.ident == "v1" {
                            vec!["Gone"]
                        } else {
                            vec![]
                        }
                    );
                }
                hir::AttrRaisesExpr::SetExcepExpr(set) => assert_eq!(names(&set.expr.0), expected),
                _ => panic!("both declared attribute contracts must survive"),
            }
        }
        assert_eq!(attributes, 2);
        let batch = body
            .iter()
            .find_map(|d| match d {
                hir::Definition::TypeDcl(hir::TypeDcl::ConstrTypeDcl(
                    hir::ConstrTypeDcl::StructDcl(s),
                )) if s.ident == "Batch" => Some(s),
                _ => None,
            })
            .unwrap();
        let hir::TypeSpec::SequenceType(sequence) = &batch.member[0].ty else {
            panic!("sequence")
        };
        let hir::TypeSpec::MapType(map) = &batch.member[1].ty else {
            panic!("map")
        };
        for bound in [&sequence.len, &map.len] {
            assert!(
                matches!(bound, Some(hir::PositiveIntConst(hir::ConstExpr::ScopedName(name))) if name.name == ["LIMIT"] && !name.is_root)
            );
        }
        let hir::Declarator::ArrayDeclarator(grid) = &batch.member[2].ident[0] else {
            panic!("grid")
        };
        assert_eq!(
            grid.len
                .iter()
                .map(|n| hir::const_expr_to_i64(&n.0))
                .collect::<Vec<_>>(),
            [Some(2), Some(3)]
        );
        assert!(
            body.iter()
                .any(|d| matches!(d, hir::Definition::TypeDcl(hir::TypeDcl::NativeDcl(_))))
        );
        assert!(body.iter().any(|d| matches!(
            d,
            hir::Definition::InterfaceDcl(hir::InterfaceDcl {
                decl: hir::InterfaceDclInner::InterfaceForwardDcl(_),
                ..
            })
        )));
    }
}

#[test]
fn retirement_applies_to_standalone_declaration_kinds_and_keeps_surviving_bit_positions() {
    let spec = lower(
        r#"
@version enum Versions { v1, v2 };
@removed(Versions::v2) native Token;
@removed(Versions::v2) union Pending;
@removed(Versions::v2) interface Forward;
@removed(Versions::v2) interface Old {};
@removed(Versions::v2) enum Kind { One };
@removed(Versions::v2) bitset Bits { bitfield<1> enabled; };
@removed(Versions::v2) bitmask Mask { One };
bitmask Flags { @removed(Versions::v2) Old, New };
"#,
    );
    let record = VersioningRecord::analyze(&spec).unwrap().unwrap();
    let v2 = record.timeline().iter().nth(1).unwrap().0;
    let projected = record.project(&spec, v2).unwrap();
    let module = projected
        .0
        .iter()
        .find_map(|d| match d {
            hir::Definition::ModuleDcl(m) => Some(m),
            _ => None,
        })
        .unwrap();
    assert_eq!(module.definition.len(), 1);
    let hir::Definition::TypeDcl(hir::TypeDcl::ConstrTypeDcl(hir::ConstrTypeDcl::BitmaskDcl(
        flags,
    ))) = &module.definition[0]
    else {
        panic!("expected surviving Flags")
    };
    assert_eq!(flags.value.len(), 1);
    assert_eq!(flags.value[0].ident, "New");
    assert_eq!(flags.value[0].position, 1);
}
