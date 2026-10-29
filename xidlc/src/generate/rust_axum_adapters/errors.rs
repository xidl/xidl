//! Adapter-local projection of `raises(...)` into error variant names.

use crate::error::{IdlcError, IdlcResult};
use convert_case::{Case, Casing};
use xidl_parser::rest_hir::{HttpOperation, RestHirDocument};

/// Compute the enum variant name for each raised exception, mirroring the
/// ordinary interface generator: bare `Pascal` when unambiguous, otherwise
/// `Http<Status><Name>` to disambiguate.
///
/// The adapter builds its typed error match arms without reaching into the
/// sibling interface plugin's `RaisesContext`.
pub(crate) fn error_variant_names(
    http_op: &HttpOperation,
    document: &RestHirDocument,
) -> IdlcResult<Vec<String>> {
    let variants = http_op
        .meta
        .raises
        .iter()
        .map(|refer| rust_ident(&refer.ident).to_case(Case::Pascal))
        .collect::<Vec<_>>();
    let qualify = variants.iter().any(|name| name == "Framework")
        || variants
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            != variants.len();
    http_op
        .meta
        .raises
        .iter()
        .map(|refer| {
            let exception = document
                .document
                .exceptions
                .iter()
                .find(|value| value.ident == refer.ident && value.module_path == refer.module_path)
                .ok_or_else(|| {
                    IdlcError::rpc(format!(
                        "raises('{}') does not resolve to a declared exception",
                        refer.ident
                    ))
                })?;
            let name = rust_ident(&exception.ident).to_case(Case::Pascal);
            Ok(if qualify {
                format!("Http{}{name}", exception.status)
            } else {
                name
            })
        })
        .collect()
}

fn rust_ident(name: &str) -> String {
    name.strip_prefix("r#").unwrap_or(name).to_string()
}
