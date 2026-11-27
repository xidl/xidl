use super::render;

#[test]
fn typescript_adapters_export_versions_and_interface_dispatchers() {
    let source = r#"
        @version(in = "header", name = "X-Version")
        enum ApiVersion {
            @rename("2026-01-01") v1,
            @rename("2026-06-01") v2
        };
        struct Item {
            string id;
            @added(ApiVersion::v2) @optional string title;
        };
        interface Items {
            @get(path = "/items/{id}")
            Item get(@path string id);
            @post(path = "/items")
            Item create(@body Item item);
        };
    "#;
    let files = render(source, "items.idl");
    assert_eq!(files.len(), 1);
    let compact = files[0].1.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        compact.contains(
            r#"export const VERSIONS = [{ id: "v1", value: "2026-01-01" }, { id: "v2", value: "2026-06-01" }] as const;"#
        ) || compact.contains(
            r#"export const VERSIONS = [ { id: "v1", value: "2026-01-01" }, { id: "v2", value: "2026-06-01" } ] as const;"#
        ),
        "{compact}"
    );
    assert!(
        compact.contains(r#"export type VersionId = (typeof VERSIONS)[number]["id"];"#),
        "{compact}"
    );
    assert!(
        compact.contains(
            "export const ItemsDispatchers = { _get: dispatchItemsGet, create: dispatchItemsCreate, } as const;"
        ),
        "{compact}"
    );
}

#[test]
fn typescript_adapters_export_scoped_interface_dispatchers_and_reject_reserved_symbol_collisions() {
    let scoped = r#"
        @version enum V { v1, v2 };
        module catalog {
            struct Item { string id; @added(V::v2) @optional string title; };
            interface Items {
                @get(path = "/items/{id}")
                Item get(@path string id);
            };
        };
    "#;
    let files = render(scoped, "catalog.idl");
    assert_eq!(files.len(), 1);
    let compact = files[0].1.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        compact.contains(
            "export const catalog_ItemsDispatchers = { _get: dispatchcatalog_ItemsGet, } as const;"
        ),
        "{compact}"
    );

    let colliding = r#"
        @version enum V { v1, v2 };
        struct Item { string id; @added(V::v2) @optional string title; };
        interface VERSIONS {
            @get(path = "/items")
            Item get();
        };
    "#;
    // `VERSIONS` as an interface generates `VERSIONSDispatchers`, whereas a type dispatcher or
    // decision colliding with `VERSIONS` / `VersionId` must be rejected by symbol validation.
    let files = render(colliding, "ok.idl");
    assert_eq!(files.len(), 1);
}

#[test]
fn distinct_scoped_interfaces_with_the_same_rendered_binding_name_are_rejected() {
    let source = r#"
        @version enum V { v1 };
        module a {
            interface b_c {
                @get(path = "/one") long first();
            };
        };
        module a_b {
            interface c {
                @get(path = "/two") long second();
            };
        };
    "#;
    let error = xidlc::generate_from_source("typescript-rest-adapters", source, Default::default())
        .err()
        .expect("colliding interface bindings must fail")
        .to_string();
    assert!(
        error.contains("generated symbol a_b_cDispatchers is ambiguous"),
        "{error}"
    );
}
