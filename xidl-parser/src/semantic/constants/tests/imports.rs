use super::{integer, project, values};
use crate::hir::{FsIncludeResolver, ParserProperties, Specification};
use crate::semantic::{INTEGER_IMPORTS_PROPERTY, IntegerExports, IntegerImports};
use crate::versioning::VersioningRecord;

fn source(source: &str) -> Specification {
    let typed = crate::parser::parser_text(source).expect("dependency IDL syntax");
    Specification::source_from_typed_ast_with_resolver(
        typed,
        "dependency.idl",
        &mut FsIncludeResolver,
    )
    .expect("dependency source declarations")
}

fn exports(input: &str) -> IntegerExports {
    let spec = source(input);
    let mut exports = IntegerExports::default();
    if let Some(record) = VersioningRecord::analyze(&spec).expect("dependency timeline") {
        for (id, definition) in record.timeline().iter() {
            let projected = record.project_source(&spec, id).expect("source projection");
            exports.record(&projected, Some(definition));
        }
    } else {
        exports.record(&spec, None);
    }
    exports
}

fn properties(imports: &IntegerImports) -> ParserProperties {
    [(
        INTEGER_IMPORTS_PROPERTY.into(),
        serde_json::to_value(imports).unwrap(),
    )]
    .into_iter()
    .collect()
}

#[test]
fn imported_values_and_aliases_keep_their_declaring_version_and_scope() {
    let dependency = exports(
        r#"
        @version enum V { @rename("old") v1, v2 };
        const unsigned long MASK = 0xfffffffd;
        module policy {
            typedef unsigned long Word;
            const Word MASK = 0xfffffffc;
            const Word LOCAL = ~MASK;
            const Word ROOTED = ~::MASK;
        };
        "#,
    );
    let mut imports = IntegerImports::new(["dep".into()]);
    imports.bind("dep", &dependency, []);
    let before = serde_json::to_value(&imports).unwrap();
    let spec = project(
        r#"
        const unsigned long MASK = 10;
        const dep::v1::policy::Word LOCAL = dep::v1::policy::LOCAL;
        const unsigned long ROOTED = dep::old::policy::ROOTED;
        const unsigned long long WIDE = dep::v1::MASK + 3;
        typedef long Slots[dep::v1::policy::ROOTED];
        "#,
        properties(&imports),
    );
    let values = values(&spec);
    assert_eq!(values["LOCAL"], integer(3));
    assert_eq!(values["ROOTED"], integer(2));
    assert_eq!(values["WIDE"], integer(4_294_967_296));
    let crate::hir::Definition::TypeDcl(crate::hir::TypeDcl::TypedefDcl(alias)) =
        spec.0.last().unwrap()
    else {
        panic!("Slots typedef");
    };
    let crate::hir::Declarator::ArrayDeclarator(slots) = &alias.decl[0] else {
        panic!("Slots fixed array");
    };
    assert_eq!(serde_json::to_value(&slots.len[0].0).unwrap(), integer(2));
    assert_eq!(serde_json::to_value(&imports).unwrap(), before);
}

#[test]
fn transitive_constants_use_only_the_imports_declared_by_their_owner() {
    let base = exports("const unsigned long MASK = 0xffffffff;");
    let middle = exports("const unsigned long long LIMIT = base::MASK + 1;");
    let mut imports = IntegerImports::new(["middle".into(), "base".into()]);
    imports.bind("base", &base, []);
    imports.bind("middle", &middle, ["base".into()]);
    let source = "const unsigned long long RESULT = middle::LIMIT + 1;";
    assert_eq!(
        values(&project(source, properties(&imports)))["RESULT"],
        integer(4_294_967_297)
    );

    imports.bind("middle", &middle, []);
    let raw = project(source, ParserProperties::new());
    assert_eq!(values(&project(source, properties(&imports))), values(&raw));
}

#[test]
fn versions_do_not_share_cached_constants_and_unselected_values_are_not_evaluated() {
    let source = r#"
        @version enum V { v1, v2 };
        @removed(V::v2) module policy { const unsigned long MASK = 0xfffffffd; };
        @added(V::v2) module policy { const unsigned long MASK = 0xfffffffc; };
    "#;
    let mut imports = IntegerImports::new(["dep".into()]);
    imports.bind("dep", &exports(source), []);
    let spec = project(
        "const long OLD = ~dep::v1::policy::MASK; const long NEW = ~dep::v2::policy::MASK;",
        properties(&imports),
    );
    assert_eq!(values(&spec)["OLD"], integer(2));
    assert_eq!(values(&spec)["NEW"], integer(3));

    let changed = source.replace("0xfffffffc", "1 / 0");
    imports.bind("dep", &exports(&changed), []);
    let old = project(
        "const long OLD = ~dep::v1::policy::MASK;",
        properties(&imports),
    );
    assert_eq!(values(&old)["OLD"], integer(2));
}

#[test]
fn imported_cycles_and_arithmetic_errors_name_the_failing_contract() {
    for (dependency, consumer, expected) in [
        (
            "const unsigned long long LIMIT = 4294967296;",
            "const unsigned long BROKEN = dep::LIMIT;",
            &["BROKEN", "32-bit unsigned"][..],
        ),
        (
            "const unsigned long LIMIT = 0xffffffff;",
            "const unsigned long BROKEN = dep::LIMIT + 1;",
            &["BROKEN", "32-bit unsigned"][..],
        ),
        (
            "const long LEFT = RIGHT; const long RIGHT = LEFT;",
            "const long BROKEN = dep::LEFT;",
            &["dep::LEFT"][..],
        ),
        (
            "typedef Second First; typedef First Second; const First LIMIT = 1;",
            "const long BROKEN = dep::LIMIT;",
            &["dep::First"][..],
        ),
    ] {
        let mut imports = IntegerImports::new(["dep".into()]);
        imports.bind("dep", &exports(dependency), []);
        let typed = crate::parser::parser_text(consumer).expect("consumer IDL syntax");
        let error =
            Specification::project_typed_ast_versions(typed, properties(&imports), "consumer.idl")
                .expect_err("invalid imported integer contract")
                .to_string();
        for fragment in expected {
            assert!(error.contains(fragment), "{error}");
        }
    }
}

#[test]
fn version_maps_evaluate_imported_array_sizes_after_selecting_each_version() {
    let spec = source(
        r#"
        @version enum V { v1, v2 };
        @removed(V::v2) module cfg {
            const unsigned long WIDTH = dep::TWO;
            typedef long Slots[WIDTH];
        };
        @added(V::v2) module cfg {
            const unsigned long WIDTH = dep::THREE;
            typedef long Slots[WIDTH];
        };
    "#,
    );
    let record = VersioningRecord::analyze(&spec).unwrap().unwrap();
    let mut imports = IntegerImports::new(["dep".into()]);
    imports.bind(
        "dep",
        &exports("const unsigned long TWO = 2; const unsigned long THREE = 3;"),
        [],
    );
    let map = crate::versioning::map::from_spec_with_http(
        &spec,
        &record,
        crate::versioning::map::MapOptions {
            contract: "arrays.idl".into(),
            include_http_operations: false,
            integer_imports: imports,
        },
    )
    .expect("per-version imported array contracts");
    assert!(map.steps[0].types.contains_key("cfg::Slots"));
    insta::assert_debug_snapshot!("imported_array_version_map", map);
}

#[test]
fn malformed_import_facts_report_the_metadata_contract() {
    let typed = crate::parser::parser_text("const long VALUE = 1;").unwrap();
    let error = Specification::project_typed_ast_versions(
        typed,
        [(INTEGER_IMPORTS_PROPERTY.into(), true.into())]
            .into_iter()
            .collect(),
        "constants.idl",
    )
    .expect_err("invalid integer import facts")
    .to_string();
    assert!(
        error.contains("integer import declarations metadata"),
        "{error}"
    );
}
