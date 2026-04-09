use super::pragma::trim_pragma_value;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn resolve_include_path(
    current_file: &Path,
    include: &crate::typed_ast::PreprocInclude,
) -> crate::error::ParserResult<PathBuf> {
    let raw = match &include.path {
        crate::typed_ast::PreprocIncludePath::StringLiteral(value) => trim_pragma_value(value),
        crate::typed_ast::PreprocIncludePath::SystemLibString(value) => {
            return Err(crate::error::ParseError::Message(format!(
                "unsupported include path syntax {value}; only string literal includes are supported"
            )));
        }
        crate::typed_ast::PreprocIncludePath::Identifier(value) => {
            return Err(crate::error::ParseError::Message(format!(
                "unsupported include identifier '{}'; only string literal includes are supported",
                value.0
            )));
        }
    };

    let base = current_file.parent().unwrap_or_else(|| Path::new("."));
    let path = normalize_path(&base.join(raw));
    if !path.is_file() {
        return Err(crate::error::ParseError::Message(format!(
            "include path '{}' does not exist",
            path.display()
        )));
    }
    Ok(path)
}

pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    fs::canonicalize(&path).unwrap_or(path)
}
