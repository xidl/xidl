#![cfg(feature = "gen-rust-axum")]

use std::{collections::HashMap, path::Path};

#[test]
fn optional_response_cookies_preserve_scalar_presence() {
    let source = r#"
#pragma xidlc package response_cookies
struct Preference { string label; };
interface Catalog {
    @get(path = "/catalog")
    string read(
        @cookie @optional out string bookmark,
        @cookie @optional out boolean consent,
        @cookie @optional out int64 revision,
        @cookie @optional out Preference preference
    );
    @put(path = "/catalog/preference")
    void remember(@cookie @optional out string bookmark);
};
"#;
    let files = xidlc::driver::Generator::new("rust-axum".into())
        .generate_from_idl(
            source,
            Path::new("response_cookies.idl"),
            HashMap::from([
                ("enable_client".into(), true.into()),
                ("enable_server".into(), true.into()),
            ]),
        )
        .expect("optional response cookie client and server generation");
    assert_eq!(files.len(), 1);
    insta::assert_snapshot!("optional_scalar_response_cookies", files[0].content());
}
