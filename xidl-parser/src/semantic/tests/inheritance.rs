use super::parse;
use crate::hir::Specification;

fn try_parse(source: &str) -> crate::error::ParserResult<Specification> {
    let typed = crate::parser::parser_text(source).expect("parse typed ast");
    Specification::from_typed_ast_with_path(typed, "inheritance.idl")
}

fn struct_named<'a>(spec: &'a Specification, name: &str) -> &'a crate::hir::StructDcl {
    fn find<'a>(
        defs: &'a [crate::hir::Definition],
        name: &str,
    ) -> Option<&'a crate::hir::StructDcl> {
        for def in defs {
            match def {
                crate::hir::Definition::ConstrTypeDcl(crate::hir::ConstrTypeDcl::StructDcl(s))
                | crate::hir::Definition::TypeDcl(crate::hir::TypeDcl::ConstrTypeDcl(
                    crate::hir::ConstrTypeDcl::StructDcl(s),
                )) if s.ident == name => {
                    return Some(s);
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
    find(&spec.0, name).expect("struct declaration")
}

#[test]
fn inherited_members_are_flat_with_base_wire_names() {
    let spec = parse(
        r#"
        @rename_all("camelCase")
        struct UserRef {
            string user_id;
            @optional string avatar_url;
        };
        struct Participant : UserRef {
            string role_name;
        };
        "#,
    );
    let child = struct_named(&spec, "Participant");
    let names = child
        .inherited
        .iter()
        .flat_map(|member| member.ident.iter())
        .map(|decl| match decl {
            crate::hir::Declarator::SimpleDeclarator(value) => value.0.as_str(),
            crate::hir::Declarator::ArrayDeclarator(value) => value.ident.as_str(),
        })
        .collect::<Vec<_>>();
    assert_eq!(names, ["user_id", "avatar_url"]);
    let wire_names = child
        .inherited
        .iter()
        .flat_map(|member| {
            member.ident.iter().map(move |decl| {
                let raw = match decl {
                    crate::hir::Declarator::SimpleDeclarator(value) => value.0.as_str(),
                    crate::hir::Declarator::ArrayDeclarator(value) => value.ident.as_str(),
                };
                crate::hir::effective_wire_name(raw, &member.annotations, &child.annotations)
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(wire_names, ["userId", "avatarUrl"]);
    assert_eq!(child.member.len(), 1);
}

#[test]
fn inherited_field_names_must_not_repeat() {
    let error = try_parse(
        r#"
        struct Base { string id; };
        struct Derived : Base { string id; };
        "#,
    )
    .expect_err("duplicate inherited field should fail");
    assert!(
        error
            .to_string()
            .contains("duplicates a field declared by an ancestor")
    );
}

#[test]
fn inheritance_cycles_are_rejected() {
    let error = try_parse(
        r#"
        struct A : B {};
        struct B : A {};
        "#,
    )
    .expect_err("inheritance cycle should fail");
    assert!(error.to_string().contains("inheritance cycle"));
}

#[test]
fn unknown_base_structs_are_rejected() {
    let error = try_parse(
        r#"
        struct Derived : Missing {};
        "#,
    )
    .expect_err("unknown base should fail");
    assert!(error.to_string().contains("which is not a complete struct"));
}
