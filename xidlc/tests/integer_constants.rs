#![cfg(feature = "gen-rust")]

use std::collections::HashMap;
use std::path::Path;

fn selected(source: &str, version: &str) -> String {
    xidlc::driver::Generator::new("rust".into())
        .generate_from_idl(
            source,
            Path::new("constants.idl"),
            HashMap::from([("api_version".into(), version.into())]),
        )
        .expect("generate selected integer constants")
        .into_iter()
        .map(|file| file.content().to_owned())
        .collect()
}

#[test]
fn changing_head_constants_preserves_the_pinned_historical_artifact() {
    let source = r#"
        @version enum V { v1, v2 };
        @removed(V::v2) module policy {
            const unsigned long MASK = 0xfffffffd;
            const long WIDTH = ~MASK;
            typedef long Coordinates[WIDTH];
        };
        @added(V::v2) module policy {
            const unsigned long MASK = 0xfffffffc;
            const long WIDTH = ~MASK;
            typedef long Coordinates[WIDTH];
        };
    "#;
    let changed = source.replace("0xfffffffc", "0xfffffffb");
    let historical = selected(source, "v1");
    assert_eq!(historical, selected(&changed, "v1"));
    assert!(historical.contains("pub const WIDTH: i32 = 2;"));
    assert!(historical.contains("[i32; 2]"));
    let head = selected(source, "v2");
    let changed_head = selected(&changed, "v2");
    assert_ne!(head, changed_head);
    assert!(head.contains("pub const WIDTH: i32 = 3;"));
    assert!(changed_head.contains("pub const WIDTH: i32 = 4;"));
}
