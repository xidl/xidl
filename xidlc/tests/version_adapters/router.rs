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
    let adapters = &files[0].1;
    assert!(
        adapters.contains(
            r#"export const VERSIONS = [{ id: "v1", value: "2026-01-01" }, { id: "v2", value: "2026-06-01" }] as const;"#
        ),
        "{adapters}"
    );
    assert!(
        adapters.contains(r#"export type VersionId = (typeof VERSIONS)[number]["id"];"#),
        "{adapters}"
    );
    assert!(
        adapters.contains("export const ItemsDispatchers ="),
        "{adapters}"
    );
    assert!(adapters.contains("get: dispatchItemsGet,"), "{adapters}");
    assert!(
        adapters.contains("create: dispatchItemsCreate,"),
        "{adapters}"
    );
}
