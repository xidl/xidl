use std::collections::HashSet;

use super::recursive_schema_types;
use crate::hir::Specification;

fn parse(source: &str) -> Specification {
    let typed = crate::parser::parser_text(source).expect("parse typed ast");
    Specification::from_typed_ast_with_properties(typed, Default::default())
}

fn assert_recursive(source: &str, expected: &[&str]) {
    let spec = parse(source);
    let recursive = recursive_schema_types(&spec);
    let expected = expected
        .iter()
        .map(|value| value.split("::").map(str::to_string).collect::<Vec<_>>())
        .collect::<HashSet<_>>();
    assert_eq!(recursive, expected, "unexpected recursive schema types");
}

#[test]
fn self_reference_through_sequence_is_recursive() {
    assert_recursive(
        r#"
        struct Node {
            string id;
            @optional
            sequence<Node> children;
        };
        "#,
        &["Node"],
    );
}

#[test]
fn direct_self_reference_is_recursive() {
    assert_recursive(
        r#"
        struct Node {
            @optional Node next;
        };
        "#,
        &["Node"],
    );
}

#[test]
fn mutual_reference_is_recursive() {
    assert_recursive(
        r#"
        struct Left {
            Right right;
        };
        struct Right {
            Left left;
        };
        "#,
        &["Left", "Right"],
    );
}

#[test]
fn typedef_alias_cycle_through_sequence_is_recursive() {
    assert_recursive(
        r#"
        typedef sequence<Node> NodeList;
        struct Node {
            @optional sequence<NodeList> children;
        };
        "#,
        &["Node", "NodeList"],
    );
}

#[test]
fn union_case_reference_is_recursive() {
    assert_recursive(
        r#"
        union TreeNode switch (long) {
            case 1: sequence<TreeNode> children;
        };
        "#,
        &["TreeNode"],
    );
}

#[test]
fn module_nested_recursion_uses_canonical_path() {
    assert_recursive(
        r#"
        module graph {
            struct Node {
                @optional sequence<Node> children;
            };
        };
        "#,
        &["graph::Node"],
    );
}

#[test]
fn plain_structs_are_not_recursive() {
    assert_recursive(
        r#"
        struct A {
            long value;
        };
        struct B {
            A a;
        };
        "#,
        &[],
    );
}

#[test]
fn typedef_chain_into_cycle_marks_cycle_members_only() {
    assert_recursive(
        r#"
        typedef Node NodeAlias;
        struct Node {
            @optional sequence<Node> children;
        };
        "#,
        &["Node"],
    );
}

#[test]
fn recursive_exception_is_detected() {
    assert_recursive(
        r#"
        exception NodeError {
            string message;
            @optional
            sequence<NodeError> causes;
        };
        "#,
        &["NodeError"],
    );
}

#[test]
fn union_case_inline_struct_cycle_is_recursive() {
    assert_recursive(
        r#"
        union Tree switch (long) {
            case 1:
                struct Item {
                    @optional Tree parent;
                    sequence<Tree> children;
                } item;
        };
        "#,
        &["Tree"],
    );
}

#[test]
fn union_case_inline_struct_into_typedef_cycle_is_recursive() {
    assert_recursive(
        r#"
        typedef sequence<Tree> TreeList;
        union Tree switch (long) {
            case 1:
                struct Item {
                    @optional Tree parent;
                    TreeList children;
                } item;
        };
        "#,
        &["Tree", "TreeList"],
    );
}
