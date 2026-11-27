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
    /// Collects every import `specification` declares, modules included.
    pub(super) fn of(specification: &typed_ast::Specification) -> Self {
        let mut decls = Vec::new();
        Self::collect(&specification.0, &mut decls);
        Self { decls }
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
