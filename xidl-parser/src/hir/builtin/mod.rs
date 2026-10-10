use super::Definition;
use crate::error::ParserResult;

#[cfg(test)]
mod tests;

/// Compiler declarations for HTTP union discriminators, never emitted as models.
pub(crate) struct HttpBuiltins;

impl HttpBuiltins {
    pub(crate) fn load() -> ParserResult<Vec<Definition>> {
        let source = include_str!("../../../builtin/http.idl");
        let typed = crate::parser::parser_text(source)?;
        let mut definitions = Vec::new();
        super::spec::collect_defs(typed.0, &mut definitions);
        Ok(definitions)
    }
}
