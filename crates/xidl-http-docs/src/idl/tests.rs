use super::*;
use xidlc::generate::hir_gen::HirGen;
use xidlc::jsonrpc::{Artifact, Codegen, CodegenInput};

fn sources(source: &str) -> BTreeMap<String, IdlSource> {
    let props = serde_json::from_value(serde_json::json!({
        "idl": source, "target_lang": "http-docs", "hir_kind": "http",
        "version_layout": "per_version"
    }))
    .unwrap();
    HirGen
        .generate(
            CodegenInput::RpcHir(hir::Specification(vec![])),
            "source.idl".into(),
            props,
        )
        .unwrap()
        .into_iter()
        .map(|artifact| {
            let Artifact::RestHir(data) = artifact else {
                panic!("expected HTTP HIR")
            };
            let version = data
                .props
                .get("api_version")
                .and_then(serde_json::Value::as_str);
            (
                version.unwrap_or("unversioned").into(),
                IdlSource::new(&data.rest_hir, version).unwrap(),
            )
        })
        .collect()
}

#[test]
fn projected_source_keeps_business_owners_and_removes_only_version_namespaces() {
    let versions = sources(
        r#"
        @version(in = "header", name = "X-Version") enum ApiVersion { v1, v2 };
        module Business {
            struct Item;
            struct Item { string id; @added(ApiVersion::v2) string note; };
            typedef Item Alias, Items[2];
            interface Store {
                typedef string Local;
                @get(path = "/items/{id}") Item read(@path string id);
                @get(path = "/names") Local names();
                readonly attribute string status, hidden;
            };
        };
    "#,
    );
    let old = &versions["v1"];
    let new = &versions["v2"];
    let item = new.declaration("Business::Item").unwrap();
    assert!(item.contains("module Business"), "{item}");
    assert!(item.contains("string note;"));
    assert!(!old.declaration("Business::Item").unwrap().contains("note"));
    let operation = new.declaration("Business::Store::read").unwrap();
    assert_eq!(old.declaration("Business::Store::read").unwrap(), operation);
    assert!(operation.contains("interface Store"));
    assert!(!operation.contains("v2::") && !operation.contains("v1::"));
    assert!(!operation.contains("names("));
    let alias = new.declaration("Business::Items").unwrap();
    assert!(alias.contains("Items[2]"), "{alias}");
    assert!(!alias.contains(" Alias"));
    let local = new.declaration("Business::Store::Local").unwrap();
    assert!(local.contains("interface Store") && local.contains("typedef string Local;"));
    let attribute = new
        .declaration("Business::Store::get_attribute_status")
        .unwrap();
    assert!(
        attribute.contains("readonly attribute string status;"),
        "{attribute}"
    );
    assert!(!attribute.contains("hidden"));
    assert!(new.declaration("Business::Missing").is_none());
}

#[test]
fn unversioned_selection_preserves_source_annotations_and_attribute_identity() {
    let versions = sources(
        r#"
        struct Item { @rename("wire_name") string name; };
        typedef struct Inner {} Alias;
        interface Store {
            @get(path = "/items") Item read();
            attribute string status, hidden;
        };
    "#,
    );
    let source = &versions["unversioned"];
    assert!(
        source
            .declaration("Item")
            .unwrap()
            .contains("@rename(\"wire_name\")")
    );
    let getter = source.declaration("Store::get_attribute_status").unwrap();
    assert_eq!(
        source.declaration("Store::set_attribute_status").unwrap(),
        getter
    );
    assert!(getter.contains("attribute string status;"));
    assert!(!getter.contains("hidden"));
    let inner = source.declaration("Inner").unwrap();
    assert!(inner.contains("struct Inner") && !inner.contains("typedef"));
    xidl_parser::parser::parser_text(&inner).unwrap();
    let alias = source.declaration("Alias").unwrap();
    assert!(alias.contains("typedef struct Inner") && alias.contains("Alias;"));
    xidl_parser::parser::parser_text(&alias).unwrap();
}
