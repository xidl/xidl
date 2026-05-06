pub mod doc;
pub mod filter;

pub use doc::doc_lines_from_annotations;
#[cfg(feature = "gen-typescript")]
pub use filter::typescript_format_filter;
pub use filter::{format_timestamp_filter, rust_format_filter};

use convert_case::{Case, Casing};

pub fn go_package_name(value: &str) -> String {
    let mut out = value.to_case(Case::Snake);
    out = out.replace('-', "_");
    if out.is_empty() {
        "xidl".to_string()
    } else {
        out
    }
}
