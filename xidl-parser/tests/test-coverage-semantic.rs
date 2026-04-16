use xidl_parser::hir::{Definition, Specification, TypeDcl};
use xidl_parser::parser::parser_text;

#[test]
fn converts_semantic_matrix_without_interface_expansion() {
    let typed = parser_text(
        r#"
        #pragma xidlc package demo.pkg
        #pragma xidlc openapi version "3.1.0"

        bitset Flags {
            bitfield<1, boolean> ready active;
            bitfield<2, octet> raw;
            bitfield<3, int8> signed_value;
            bitfield<4, uint8> unsigned_value;
        };

        bitmask Permissions { @key read, write };

        union Choice switch (long) {
            case 0: @id(9) long same;
            case 1: long same;
            default: short other;
        };

        typedef sequence<long, 2> LongSeq, LongArray[4];
        native NativeThing;

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
    )
    .expect("parse should succeed");

    let hir = Specification::from_typed_ast_with_properties(
        typed,
        [("expand_interface".to_string(), serde_json::json!(false))]
            .into_iter()
            .collect(),
    );

    assert!(hir.0.iter().any(|def| matches!(def, Definition::Pragma(_))));
    assert!(
        hir.0
            .iter()
            .any(|def| matches!(def, Definition::InterfaceDcl(_)))
    );
    assert!(
        hir.0
            .iter()
            .any(|def| matches!(def, Definition::TypeDcl(_)))
    );

    let typedef = hir
        .0
        .iter()
        .find_map(|def| match def {
            Definition::TypeDcl(value) => Some(value),
            _ => None,
        })
        .expect("typedef");
    assert!(matches!(typedef, TypeDcl::ConstrTypeDcl(_)));
}
