#![cfg(all(
    feature = "gen-rust-axum-adapters",
    feature = "gen-typescript-rest-adapters"
))]

use std::{collections::HashMap, path::Path};

const MODULE_MATCHING_VERSION: &str = r#"
@version enum V { v1, v2 };
typedef long Identifier;
const long WIDTH = 2;
module v1 {
    typedef string Identifier;
    const long WIDTH = 3;
    struct Base { ::v1::Identifier id; };
};
struct Payload : ::v1::Base { long parts[::v1::WIDTH]; };
interface Store {
    @post(path="/payload") ::Payload save(@body ::Payload value);
};
"#;

#[test]
fn source_modules_named_like_versions_are_not_unwrapped_twice() {
    for (target, fragments) in [
        (
            "rust-axum-adapters",
            vec![
                "v1::v1::Identifier",
                "v2::v1::Identifier",
                "array.len() != (3) as usize",
            ],
        ),
        (
            "typescript-rest-adapters",
            vec!["types.v1.v1.Identifier", "types.v2.v1.Identifier"],
        ),
    ] {
        let files = xidlc::driver::Generator::new(target.into())
            .generate_from_idl(
                MODULE_MATCHING_VERSION,
                Path::new("roots.idl"),
                HashMap::new(),
            )
            .expect("root source module and version wrapper are distinct");
        let output = files[0].content();
        for fragment in fragments {
            assert!(output.contains(fragment), "{target} lost {fragment}");
        }
    }
}
