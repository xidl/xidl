use crate::error::{ParseError, ParserResult};
use crate::typed_ast::Definition;

use super::{AnnotationKind, check::Checker};

impl AnnotationKind {
    /// Validates one built-in contract before lowering loses source locations.
    pub(crate) fn validate_source(self, definitions: &[Definition]) -> ParserResult<()> {
        let mut checker = Checker::for_annotation(self);
        checker.definitions(definitions);
        if let Some(diagnostic) = checker.finish().first() {
            return Err(ParseError::Message(format!(
                "xidl annotation at bytes {}..{}: {}",
                diagnostic.span.start, diagnostic.span.end, diagnostic.message
            )));
        }
        Ok(())
    }
}
