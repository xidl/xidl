use super::Definition;
use crate::error::ParserResult;

#[cfg(test)]
mod tests;

/// Resolves shared runtime declarations without adding them to user models.
pub(crate) struct HttpBuiltins;

impl HttpBuiltins {
    pub(crate) fn load() -> ParserResult<Vec<Definition>> {
        let source = xidl_http::ContentType::IDL;
        let typed = crate::parser::parser_text(source)?;
        let mut definitions = Vec::new();
        super::spec::collect_defs(typed.0, &mut definitions);
        Ok(definitions)
    }
}
