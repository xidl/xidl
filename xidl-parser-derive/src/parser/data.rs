use super::fields::Fields;

/// Enum-or-struct shape of a Parser derive input.
#[derive(Debug)]
pub enum Data<V, F> {
    /// Enum input with one entry per variant.
    Enum(Vec<V>),
    /// Struct input with its field collection.
    Struct(Fields<F>),
}
