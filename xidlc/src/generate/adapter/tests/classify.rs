//! Classification edge cases pinned on hand-built surfaces.

use super::*;

#[test]
fn an_in_place_type_change_pairs_as_a_replacement() {
    let source = members_surface(
        TypeKind::Members,
        vec![member(
            "Parcel::quantity",
            "quantity",
            Some(i32_type()),
            false,
        )],
    );
    let target = members_surface(
        TypeKind::Members,
        vec![member(
            "Parcel::quantity",
            "quantity",
            Some(bool_type()),
            false,
        )],
    );
    let intervals = vec![interval_of("Parcel::quantity", 0, 2)];
    let upcast = convert_direction(Direction::Upcast, &source, &target, &intervals, 0, 1)
        .expect("replacement upcast");
    assert_eq!(
        hook_fields(&upcast),
        vec![
            ("quantity".to_string(), false),
            ("quantity".to_string(), true)
        ]
    );
    let downcast = convert_direction(Direction::Downcast, &target, &source, &intervals, 1, 0)
        .expect("replacement downcast");
    assert_eq!(
        hook_fields(&downcast),
        vec![("quantity".to_string(), false)]
    );
    assert!(downcast.copies.is_empty());
}

fn members_surface(kind: TypeKind, members: Vec<MemberSurface>) -> TypeSurface {
    TypeSurface { kind, members }
}

fn member(
    decl_path: &str,
    wire_name: &str,
    ty: Option<hir::TypeSpec>,
    optional: bool,
) -> MemberSurface {
    let ident = decl_path
        .rsplit("::")
        .next()
        .unwrap_or(decl_path)
        .to_string();
    MemberSurface {
        decl_path: decl_path.to_string(),
        ident,
        wire_name: wire_name.to_string(),
        ty,
        optional,
        location: None,
    }
}

fn i32_type() -> hir::TypeSpec {
    hir::TypeSpec::IntegerType(hir::IntegerType::I32)
}

fn bool_type() -> hir::TypeSpec {
    hir::TypeSpec::Boolean
}

fn interval_of(path: &str, from: usize, until: usize) -> MemberInterval {
    MemberInterval {
        path: path.to_string(),
        kind: xidl_parser::versioning::IntervalKind::Member,
        from,
        until,
    }
}
