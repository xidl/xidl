mod data;
mod derive_field;
mod derive_input;
mod derived_variant;
mod fields;
mod gen_struct;
mod gen_variant;
mod style;

pub use data::Data;
pub(crate) use derive_field::DeriveField;
pub(crate) use derive_input::DeriveInput;
pub(crate) use derived_variant::DerivedVariant;
pub use fields::Fields;
pub use style::Style;

use syn::parse_macro_input;

/// Expand `#[derive(Parser)]` into a Tree-sitter mapping.
pub fn tree_sitter_parser(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let derived = match DeriveInput::from_derive_input(&input) {
        Ok(v) => v,
        Err(err) => return err.to_compile_error().into(),
    };
    derived.generate().into()
}
