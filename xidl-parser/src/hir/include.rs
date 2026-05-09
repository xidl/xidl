use crate::parser::IncludeResolver;
use std::fs;
use std::path::{Path, PathBuf};

pub struct FsIncludeResolver;

impl IncludeResolver for FsIncludeResolver {
    fn resolve(
        &mut self,
        parent_path: Option<&str>,
        path: &str,
    ) -> crate::error::ParserResult<(String, String)> {
        let parent = parent_path.map(Path::new).unwrap_or_else(|| Path::new("."));
        let base = parent.parent().unwrap_or_else(|| Path::new("."));
        let resolved_path = normalize_path(&base.join(path));

        if !resolved_path.is_file() {
            return Err(crate::error::ParseError::Message(format!(
                "include path '{}' does not exist",
                resolved_path.display()
            )));
        }

        let content = fs::read_to_string(&resolved_path).map_err(|err| {
            crate::error::ParseError::Message(format!(
                "failed to read include '{}': {err}",
                resolved_path.display()
            ))
        })?;

        Ok((resolved_path.display().to_string(), content))
    }
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
