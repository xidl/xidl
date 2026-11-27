use super::support::TempDir;
use crate::units::UnitGraph;
use std::collections::BTreeMap;
use xidl_parser::typed_ast::{
    ConstrTypeDcl, Definition, SimpleTypeSpec, StructDcl, TypeDclInner, TypeSpec,
};

#[test]
fn unit_exposes_the_typed_ast_matching_its_prepared_source() {
    let dir = TempDir::new("typed-ast-cache");
    dir.write(
        "timestamp/index.idl",
        "@version\nenum ApiVersions { v1, v2 };\nstruct Instant { string at; };\n",
    );
    let app = dir.write(
        "app.idl",
        "import \"timestamp@v1\";\nstruct Event { timestamp::Instant at; };\n",
    );

    let graph = UnitGraph::build(&[app], &BTreeMap::new()).expect("graph builds");
    let app = graph
        .units()
        .iter()
        .find(|unit| unit.prefix() == "app")
        .expect("app unit");
    let timestamp = graph
        .units()
        .iter()
        .find(|unit| unit.prefix() == "timestamp")
        .expect("timestamp unit");

    assert_eq!(app.refs().len(), 1);
    assert_eq!(app.refs()[0].qualified(), "timestamp::v1::Instant");

    let event_struct = app
        .typed_ast()
        .0
        .iter()
        .find_map(|def| match def {
            Definition::TypeDcl(type_dcl) => match &type_dcl.decl {
                TypeDclInner::ConstrTypeDcl(ConstrTypeDcl::StructDcl(StructDcl::StructDef(
                    def,
                ))) => Some(def),
                _ => None,
            },
            _ => None,
        })
        .expect("Event struct in app typed AST");
    assert_eq!(event_struct.ident.0, "Event");
    let TypeSpec::SimpleTypeSpec(SimpleTypeSpec::ScopedName(scoped)) = &event_struct.member[0].ty
    else {
        panic!("expected ScopedName member type");
    };
    assert_eq!(scoped.node_text, "timestamp::v1::Instant");

    let instant_struct = timestamp
        .typed_ast()
        .0
        .iter()
        .find_map(|def| match def {
            Definition::TypeDcl(type_dcl) => match &type_dcl.decl {
                TypeDclInner::ConstrTypeDcl(ConstrTypeDcl::StructDcl(StructDcl::StructDef(
                    def,
                ))) => Some(def),
                _ => None,
            },
            _ => None,
        })
        .expect("Instant struct in unchanged timestamp typed AST");
    assert_eq!(instant_struct.ident.0, "Instant");
}
