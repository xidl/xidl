use crate::{hir, parser::parser_text, semantic::recursive_schema_types};
use std::collections::BTreeSet;

fn lower(source: &str) -> hir::Specification {
    hir::Specification::from_typed_ast_with_properties_and_path(
        parser_text(source).unwrap(),
        Default::default(),
        "graph.idl",
    )
    .unwrap()
}

#[test]
fn schema_cycles_follow_aliases_containers_and_inline_types_without_marking_leaves() {
    let source = r#"
module graph {
    typedef map<KeyNode, ValueNode> Lookup;
    struct KeyNode { Lookup links; };
    struct ValueNode { long value; };
    typedef Pair<TemplateNode, long> Boxed;
    struct TemplateNode { Boxed box; };
    typedef struct Inline { Alias next; } Alias;
    typedef union InlineChoice switch(long) { case 1: ChoiceAlias next; } ChoiceAlias;
    typedef enum InlineState { Idle } StateAlias;
    typedef long Numbers[2];
    enum Plain { Ready };
    native Native;
    const long LIMIT = 1;
    union Tree switch(long) {
        case 1: union Branch switch(long) { case 1: Tree parent; default: long count; } branch;
        case 2: enum Mode { Leaf } mode;
    };
};
"#;
    let spec = lower(source);
    let actual: BTreeSet<_> = recursive_schema_types(&spec)
        .into_iter()
        .map(|path| path.join("::"))
        .collect();
    let expected = [
        "Lookup",
        "KeyNode",
        "Boxed",
        "TemplateNode",
        "Alias",
        "ChoiceAlias",
        "Tree",
    ]
    .into_iter()
    .map(|name| format!("graph::{name}"))
    .collect();
    assert_eq!(actual, expected);
    let restored: hir::Specification =
        serde_json::from_str(&serde_json::to_string(&spec).unwrap()).unwrap();
    assert_eq!(
        recursive_schema_types(&restored),
        recursive_schema_types(&spec)
    );
}

#[test]
fn recursive_inline_typedef_members_preserve_boxing_decisions() {
    let spec = lower(
        r#"
        typedef struct Node { @optional Node next; long value; } NodeAlias;
        typedef union Link switch(long) { case 1: Link next; default: long value; } LinkAlias;
    "#,
    );
    for definition in &spec.0 {
        let hir::Definition::TypeDcl(hir::TypeDcl::TypedefDcl(alias)) = definition else {
            panic!("typedef")
        };
        match &alias.ty {
            hir::TypedefType::ConstrTypeDcl(hir::ConstrTypeDcl::StructDcl(node)) => {
                assert!(node.member[0].recursive);
                assert!(!node.member[1].recursive);
            }
            hir::TypedefType::ConstrTypeDcl(hir::ConstrTypeDcl::UnionDef(link)) => {
                assert!(link.case[0].element.recursive);
                assert!(!link.case[1].element.recursive);
            }
            _ => panic!("inline schema"),
        }
    }
}
