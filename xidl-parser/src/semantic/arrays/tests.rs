use crate::{error::ParserResult, hir::Specification};
use serde_json::Value;
use std::collections::BTreeMap;

fn parse(source: &str) -> ParserResult<Specification> {
    let typed = crate::parser::parser_text(source).expect("legal IDL syntax");
    Specification::from_typed_ast_with_path(typed, "dimensions.idl")
}

fn dimensions(spec: &Specification) -> BTreeMap<String, Vec<i64>> {
    fn visit(value: &Value, dimensions: &mut BTreeMap<String, Vec<i64>>) {
        match value {
            Value::Object(object) => {
                if let Some(array) = object.get("ArrayDeclarator") {
                    let sizes = array["len"]
                        .as_array()
                        .expect("dimensions")
                        .iter()
                        .map(|size| {
                            size["Literal"]["IntegerLiteral"]
                                .as_str()
                                .expect("normalized integer")
                                .parse()
                                .expect("integer size")
                        })
                        .collect();
                    dimensions.insert(array["ident"].as_str().expect("array name").into(), sizes);
                }
                for value in object.values() {
                    visit(value, dimensions);
                }
            }
            Value::Array(array) => {
                for value in array {
                    visit(value, dimensions);
                }
            }
            _ => {}
        }
    }
    let mut sizes = BTreeMap::new();
    visit(
        &serde_json::to_value(spec).expect("serialize HIR"),
        &mut sizes,
    );
    sizes
}

#[test]
fn dimensions_use_declaration_scope_and_preserve_constant_declarations() {
    let mut spec = parse(
        r#"
        const long WIDTH = 2;
        module geometry {
            const long WIDTH = 3;
            typedef unsigned long Size;
            const Size COMPONENTS = ::WIDTH * 2;
            typedef long Quad[COMPONENTS], Pair[COMPONENTS / 2];
            module nested {
                typedef double Size;
                typedef long InheritedConstant[COMPONENTS];
            };
        };
        module consumers {
            const long COMPONENTS = 9;
            typedef geometry::Pair Copies[::WIDTH];
            struct Position { long coordinates[geometry::COMPONENTS / 2]; };
        };
    "#,
    )
    .expect("scoped integral constants");
    assert_eq!(
        dimensions(&spec),
        BTreeMap::from([
            ("Quad".into(), vec![4]),
            ("Pair".into(), vec![2]),
            ("InheritedConstant".into(), vec![4]),
            ("Copies".into(), vec![2]),
            ("coordinates".into(), vec![2]),
        ])
    );
    let before = serde_json::to_value(&spec).unwrap();
    assert!(
        before.to_string().contains("BinaryExpr"),
        "constant RHS remains an expression"
    );
    crate::semantic::analyze(&mut spec).expect("repeated analysis");
    assert_eq!(before, serde_json::to_value(&spec).unwrap());
}

#[test]
fn every_array_declarator_is_normalized_before_inheritance() {
    let spec = parse(
        r#"
        const long WIDTH = 2;
        typedef struct Inline { long inline_values[WIDTH]; } Alias;
        struct Base { long base_values[WIDTH]; };
        struct Derived : Base { long grid[WIDTH][WIDTH + 1]; };
        union Choice switch (boolean) {
            case TRUE: long values[WIDTH];
            default: long scalar;
        };
        exception Failure { long facts[WIDTH]; };
        interface Local {
            const long WIDTH = 3;
            typedef long InterfaceArray[WIDTH];
            exception Rejected { long local_facts[WIDTH]; };
        };
    "#,
    )
    .expect("arrays in every supported declaration position");
    assert_eq!(
        dimensions(&spec),
        BTreeMap::from([
            ("inline_values".into(), vec![2]),
            ("base_values".into(), vec![2]),
            ("grid".into(), vec![2, 3]),
            ("values".into(), vec![2]),
            ("facts".into(), vec![2]),
            ("InterfaceArray".into(), vec![3]),
            ("local_facts".into(), vec![3]),
        ])
    );
}

#[test]
fn dimensions_are_evaluated_after_selecting_the_version() {
    let source = r#"
        @version enum Versions { v1, v2 };
        @removed(Versions::v2) module geometry {
            const long WIDTH = 2;
            typedef long Coordinates[WIDTH];
        };
        @added(Versions::v2) module geometry {
            const long WIDTH = 3;
            typedef long Coordinates[WIDTH];
        };
    "#;
    for (version, size) in [("v1", 2), ("v2", 3)] {
        let typed = crate::parser::parser_text(source).expect("versioned constant declarations");
        let properties = std::collections::HashMap::from([(
            crate::versioning::API_VERSION_PROPERTY.to_string(),
            version.into(),
        )]);
        let projections =
            Specification::project_typed_ast_versions(typed, properties, "dimensions.idl")
                .expect("project constants before evaluating dimensions");
        assert_eq!(projections.len(), 1);
        let crate::rest_hir::ProjectedHir::Rpc(spec) = &projections[0].hir else {
            panic!("RPC HIR projection");
        };
        assert_eq!(dimensions(spec)["Coordinates"], [size], "{version}");
    }
}

#[test]
fn integer_operations_retain_idl_width_sign_and_unsigned_intermediates() {
    // OMG IDL 4.2 7.4.1.4.3 specifies unsigned complements and zero-fill shifts.
    for (declarations, expression, expected) in [
        ("", "+2", 2),
        ("", "(8 / 2) - 2", 2),
        ("", "(5 % 3) * 2", 4),
        ("", "(1 | 6) ^ 5", 2),
        ("", "6 & 3", 2),
        (
            "const unsigned long MASK = 0xffffffff; const unsigned long REST = ~MASK;",
            "REST + 2",
            2,
        ),
        ("const long long NEG = -2;", "(NEG >> 63) + 1", 2),
        ("const unsigned long long ONE = 1;", "(ONE << 63) >> 62", 2),
        ("const long NEG = -2;", "(~NEG) + 1", 2),
        (
            "typedef unsigned short Count; const Count TWO = 2;",
            "TWO",
            2,
        ),
        ("const octet TWO = 2;", "TWO", 2),
    ] {
        let source = format!("{declarations} typedef long Pair[{expression}];");
        let spec = parse(&source).unwrap_or_else(|error| panic!("{source}: {error}"));
        assert_eq!(dimensions(&spec)["Pair"], [expected], "{source}");
    }
}

#[test]
fn invalid_dimensions_report_the_declaration_and_underlying_contract() {
    for (declarations, expression, reason) in [
        ("", "0", "must be positive"),
        ("", "-1", "must be positive"),
        ("", "2.0", "not an integer"),
        ("const double WIDTH = 2;", "WIDTH", "not integral"),
        (
            "typedef double Size; const Size WIDTH = 2;",
            "WIDTH",
            "not integral",
        ),
        (
            "typedef long Size[2]; const Size WIDTH = 2;",
            "WIDTH",
            "not an integral alias",
        ),
        ("const short WIDTH = 65536;", "WIDTH", "16-bit signed"),
        (
            "const unsigned long WIDTH = -2;",
            "WIDTH",
            "32-bit unsigned",
        ),
        (
            "const long WIDTH = 4294967296 - 4294967294;",
            "WIDTH",
            "32-bit unsigned",
        ),
        ("const long A = B; const long B = A;", "A", "cycle"),
        (
            "typedef B A; typedef A B; const A WIDTH = 2;",
            "WIDTH",
            "cycle",
        ),
        ("struct WIDTH {};", "WIDTH", "not a constant"),
        ("", "MISSING", "does not resolve"),
        ("", "2 / 0", "invalid or overflows"),
        ("", "2 % 0", "invalid or overflows"),
        ("", "2 << 64", "invalid or overflows"),
        ("", "2 >> -1", "invalid or overflows"),
        ("", "0xffffffff + 1", "32-bit unsigned"),
        ("const long long ONE = 1;", "ONE << 63", "64-bit signed"),
        (
            "const unsigned long long ONE = 1;",
            "ONE << 64",
            "invalid or overflows",
        ),
        (
            "const unsigned long long ONE = 1;",
            "(ONE << 63) * 3",
            "64-bit unsigned",
        ),
        (
            "const unsigned long long ONE = 1;",
            "ONE << 63",
            "supported dimension range",
        ),
    ] {
        let source =
            format!("module geometry {{ {declarations} typedef long Pair[{expression}]; }};");
        let error = parse(&source).expect_err("invalid dimension").to_string();
        assert!(error.contains("geometry::Pair"), "{source}: {error}");
        assert!(error.contains(reason), "{source}: {error}");
    }
}
