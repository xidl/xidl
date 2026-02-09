use miette::NamedSource;
use tree_sitter::Node;

use crate::error::{DiagnosticError, IdlcResult};

pub fn diagnostic(node: Node, source: &str) -> IdlcResult<()> {
    if node.is_error() {
        let range = node.range();
        Err(DiagnosticError {
            src: NamedSource::new("idl", source.into()),
            bad_bit: (range.start_byte, range.end_byte - range.start_byte).into(),
        })?;
    }

    for child in node.children(&mut node.walk()) {
        diagnostic(child, source)?;
    }

    Ok(())
}
