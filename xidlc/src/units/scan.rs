use super::error::ImportError;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use xidl_parser::typed_ast;

/// One `import` declaration as written in a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ImportDecl {
    /// A quoted path, without its quotes.
    Path(String),
    /// A bare scope name.
    Scoped(String),
}

/// The `import` declarations one source file contains.
pub(super) struct DeclaredImports {
    decls: Vec<ImportDecl>,
}

impl DeclaredImports {
    /// Parses `source` and collects every import it declares, modules included.
    ///
    /// # Errors
    ///
    /// Returns [`ImportError::Parse`] when the file cannot be parsed, so a
    /// malformed unit fails before its imports are followed. Reads the source
    /// again; the parse is shared with generation.
    pub(super) fn of(
        source: &str,
        path: &Path,
        mounts: &BTreeMap<String, PathBuf>,
    ) -> Result<Self, ImportError> {
        if !source.contains("import") {
            return Ok(Self { decls: Vec::new() });
        }
        let mut resolver = crate::library::include_resolver(mounts);
        let typed =
            xidl_parser::parser::parser_text_with_resolver(source, path.to_str(), &mut *resolver)
                .map_err(|err| ImportError::Parse {
                path: path.to_path_buf(),
                reason: err.to_string(),
            })?;
        let mut decls = Vec::new();
        Self::collect(&typed.0, &mut decls);
        Ok(Self { decls })
    }

    /// The declarations, in source order.
    pub(super) fn into_decls(self) -> Vec<ImportDecl> {
        self.decls
    }

    /// Walks `definitions` and pushes every import declaration it holds.
    fn collect(definitions: &[typed_ast::Definition], out: &mut Vec<ImportDecl>) {
        for definition in definitions {
            match definition {
                typed_ast::Definition::ImportDcl(import) => out.push(match &import.scope {
                    typed_ast::ImportedScope::StringLiteral(text) => {
                        ImportDecl::Path(text.trim().trim_matches('"').to_string())
                    }
                    typed_ast::ImportedScope::ScopedName(name) => {
                        ImportDecl::Scoped(name.node_text.clone())
                    }
                }),
                typed_ast::Definition::ModuleDcl(module) => Self::collect(&module.definition, out),
                _ => {}
            }
        }
    }
}
