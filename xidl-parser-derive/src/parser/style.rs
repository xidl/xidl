/// Shape of a Rust item's fields for Tree-sitter mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Style {
    /// Unit variant or unit struct with no fields.
    Unit,
    /// Tuple variant or tuple struct with unnamed fields.
    Tuple,
    /// Struct variant or struct with named fields.
    Struct,
}
