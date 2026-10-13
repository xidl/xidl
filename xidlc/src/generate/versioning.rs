//! Guards for targets that cannot render versioned projections yet.

use crate::error::{IdlcError, IdlcResult};
use xidl_parser::hir::ParserProperties;
use xidl_parser::versioning;

/// Fails when a versioned projection reaches a target whose module rendering
/// wraps each module body separately.
///
/// Those targets would emit one `module <version>` block per body instead of
/// one module per API version, which does not compile.
pub(crate) fn reject_versioned_modules(target: &str, props: &ParserProperties) -> IdlcResult<()> {
    let versioned = props
        .get(versioning::API_VERSION_MODULES_PROPERTY)
        .and_then(serde_json::Value::as_array)
        .is_some_and(|modules| !modules.is_empty());
    if !versioned {
        return Ok(());
    }
    Err(IdlcError::fmt(format!(
        "xidl versioning: the {target} target cannot render one module per API version yet; versioned contracts are not supported for it"
    )))
}

/// Fails when a debug target receives an API version selection it ignores.
pub(crate) fn reject_version_selection(
    target: &str,
    props: &ParserProperties,
) -> Result<(), crate::jsonrpc::RpcError> {
    let selected = props.contains_key(versioning::API_VERSION_PROPERTY)
        || props.contains_key(versioning::API_VERSIONS_PROPERTY);
    if !selected {
        return Ok(());
    }
    Err(crate::jsonrpc::RpcError::new(format!(
        "xidl versioning: {target} output does not apply an API version selection; use the hir target or a generator target instead"
    )))
}
