//! TypeScript source formatting, backed by the `tsv_ts` formatter.
//!
//! `tsv_ts` prints the same layout Prettier does, with a fixed configuration:
//! tabs, single quotes, no trailing commas, and a print width of 100.

use crate::error::{IdlcError, IdlcResult};

#[cfg(test)]
mod tests;

/// Formats TypeScript source, keeping the trailing newline.
///
/// # Errors
///
/// Returns [`IdlcError::Fmt`] when `tsv_ts` rejects the source.
pub fn format_typescript_source(source: &str) -> IdlcResult<String> {
    let formatted = tsv_ts::format(source)
        .map_err(|err| IdlcError::Fmt(format!("format typescript source: {err}")))?;
    Ok(crate::fmt::helpers::ensure_trailing_newline(&formatted))
}
