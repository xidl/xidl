const TEST_CASES: &[(&str, &str)] = &[(
    "interface_dcl",
    r#"
        interface HelloWorld;
        interface HelloWorld {};

        interface HelloWorld: Parent {};
        interface HelloWorld: Parent1, Parent2, Parent3 {};

        interface A: B, C, D {
            void func1();
            void func1() raises(A);
            void func1() raises(A,B,C);
            void func1(@key in u8 attr, out u16 attr);
            void func1(in u8 attr, out u16 attr) raises(A);
            void func1(in u8 attr, out u16 attr) raises(A,B,C);
            readonly attribute u8 attr1, attr2, attr3;
            readonly attribute u8 attr4 raises(A);
            readonly attribute u8 attr5 raises(A, B, C);
            attribute u8 attr6, attr7, attr8;
            attribute u8 attr9 getraises(A);
            attribute u8 attr10 getraises(A, B, C) setraises(A);
            attribute u8 attr11 setraises(A);
        };

        interface A {
            typedef long L1; // idl 7.4.4
            const int a = 10; // idl 7.4.4
            exception A { // idl 7.4.4
                int a;
            };
            short opA(in L1 l_1);
        };
    "#,
)];

// The HIR expands operations into wrapper structs keyed by operation name,
// so the lowered fixture spells each form with a distinct operation name;
// the raw fixture above keeps the repeated spellings for the parser.
const HIR_TEST_CASES: &[(&str, &str)] = &[(
    "interface_dcl",
    r#"
        interface HelloWorld;
        interface HelloWorld {};

        interface HelloWorld: Parent {};
        interface HelloWorld: Parent1, Parent2, Parent3 {};

        interface A: B, C, D {
            void func1();
            void func2() raises(A);
            void func3() raises(A,B,C);
            void func4(@key in u8 attr, out u16 attr);
            void func5(in u8 attr, out u16 attr) raises(A);
            void func6(in u8 attr, out u16 attr) raises(A,B,C);
            readonly attribute u8 attr1, attr2, attr3;
            readonly attribute u8 attr4 raises(A);
            readonly attribute u8 attr5 raises(A, B, C);
            attribute u8 attr6, attr7, attr8;
            attribute u8 attr9 getraises(A);
            attribute u8 attr10 getraises(A, B, C) setraises(A);
            attribute u8 attr11 setraises(A);
        };

        interface A {
            typedef long L1; // idl 7.4.4
            const int a = 10; // idl 7.4.4
            exception A { // idl 7.4.4
                int a;
            };
            short opA(in L1 l_1);
        };
    "#,
)];

#[test]
fn test_typed_ast() {
    for (name, text) in TEST_CASES {
        let ast = xidl_parser::parser::parser_text(text).unwrap();
        let snapshot = format!("typed_ast__{name}");
        insta::assert_debug_snapshot!(snapshot, ast);
    }
}

#[test]
fn test_hir() {
    for (name, text) in HIR_TEST_CASES {
        let typed = xidl_parser::parser::parser_text(text).unwrap();
        let hir = xidl_parser::hir::Specification::from(typed);
        let snapshot = format!("hir__{name}");
        insta::assert_debug_snapshot!(snapshot, hir);
    }
}
