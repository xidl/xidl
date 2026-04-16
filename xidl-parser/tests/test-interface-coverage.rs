use xidl_parser::hir::{Definition, Specification, TypeDcl};
use xidl_parser::parser::parser_text;

fn parse_hir(source: &str, expand_interface: bool) -> Specification {
    let typed = parser_text(source).expect("parse should succeed");
    if expand_interface {
        Specification::from(typed)
    } else {
        Specification::from_typed_ast_with_properties(
            typed,
            [("expand_interface".to_string(), serde_json::json!(false))]
                .into_iter()
                .collect(),
        )
    }
}

fn count_defs(defs: &[Definition]) -> usize {
    defs.iter()
        .map(|def| match def {
            Definition::ModuleDcl(module) => 1 + count_defs(&module.definition),
            _ => 1,
        })
        .sum()
}

#[test]
fn rich_interface_expansion_generates_rpc_shapes_and_hash_consts() {
    let hir = parse_hir(
        r#"
        module demo {
            interface Service {
                boolean ping(
                    in boolean flag,
                    out char letter,
                    inout wchar rune
                ) raises(Failure, ::RootFailure);

                Object op_object(in Object value);
                ValueBase op_value(in ValueBase value);
                any op_any(in any value);
                long long op_i64(in int8 a, in short b, in long c, in long long d);
                unsigned long long op_u64(in uint8 a, in unsigned short b, in unsigned long c, in unsigned long long d);
                string<8> op_str(in string<4> value);
                wstring<6> op_wstr(in wstring<3> value);
                fixed<10, 2> op_fixed(in fixed<5, 1> value);
                sequence<boolean, 2> op_seq(in sequence<char> value);
                map<char, boolean, 3> op_map(in map<char, boolean> value);
                Pair<boolean, char> op_tpl(in Pair<boolean, char> value);

                readonly attribute boolean rflag;
                attribute short attr_i16, attr_i16_b;
                attribute unsigned short attr_u16;
                attribute unsigned long attr_u32;
                attribute unsigned long long attr_u64;
                attribute int8 attr_i8;
                attribute long attr_i32;
                attribute any attr_any;
                attribute Object attr_object;
                attribute ValueBase attr_value;
                attribute string<8> attr_str;
                attribute wstring<8> attr_wstr;
                attribute fixed<10, 2> attr_fixed;
                attribute sequence<boolean, 2> attr_seq;
                attribute map<char, boolean, 3> attr_map;
                attribute Pair<boolean, char> attr_tpl getraises(GetError) setraises(SetError);
            };
        };
        "#,
        true,
    );
    assert!(count_defs(&hir.0) > 5);
}

#[test]
fn disabling_interface_expansion_keeps_only_original_interface() {
    let hir = parse_hir(
        r#"
        interface Service {
            void ping();
        };
        "#,
        false,
    );

    assert_eq!(hir.0.len(), 1);
    let Definition::InterfaceDcl(interface) = &hir.0[0] else {
        panic!("expected interface");
    };
    assert!(matches!(
        interface.decl,
        xidl_parser::hir::InterfaceDclInner::InterfaceDef(_)
    ));
}

#[test]
fn interface_conversion_preserves_attr_and_raises_variants() {
    let hir = parse_hir(
        r#"
        interface Service {
            readonly attribute boolean ready raises(ReadError);
            attribute boolean state getraises(GetError) setraises(SetError);
            attribute boolean flag setraises(SetOnly);
            void ping(@get in boolean arg) raises(Failure);
            typedef long Local;
            const long ANSWER = 42;
            exception Problem {
                long code;
            };
        };
        "#,
        false,
    );

    let Definition::InterfaceDcl(interface) = &hir.0[0] else {
        panic!("expected interface");
    };
    let xidl_parser::hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
        panic!("expected definition");
    };
    let body = def.interface_body.as_ref().expect("body");
    assert!(
        body.0
            .iter()
            .any(|export| matches!(export, xidl_parser::hir::Export::OpDcl(_)))
    );
    assert!(
        body.0
            .iter()
            .any(|export| matches!(export, xidl_parser::hir::Export::AttrDcl(_)))
    );
    assert!(
        body.0
            .iter()
            .any(|export| matches!(export, xidl_parser::hir::Export::TypeDcl(_)))
    );
    assert!(
        body.0
            .iter()
            .any(|export| matches!(export, xidl_parser::hir::Export::ConstDcl(_)))
    );
    assert!(
        body.0
            .iter()
            .any(|export| matches!(export, xidl_parser::hir::Export::ExceptDcl(_)))
    );

    let attr = body
        .0
        .iter()
        .find_map(|export| match export {
            xidl_parser::hir::Export::AttrDcl(attr) => Some(attr),
            _ => None,
        })
        .expect("attr");
    assert!(matches!(
        attr.decl,
        xidl_parser::hir::AttrDclInner::ReadonlyAttrSpec(_)
    ));

    let typedef = body
        .0
        .iter()
        .find_map(|export| match export {
            xidl_parser::hir::Export::TypeDcl(value) => Some(value),
            _ => None,
        })
        .expect("typedef");
    assert!(matches!(typedef, TypeDcl::TypedefDcl(_)));
}
