use super::data::Data;
use super::derive_field::DeriveField;
use super::derived_variant::DerivedVariant;
use super::fields::Fields;
use convert_case::{Case, Casing};
use syn::LitStr;

/// Parsed Parser derive input with its Tree-sitter mapping.
#[derive(Debug)]
pub(crate) struct DeriveInput {
    pub(crate) ident: syn::Ident,
    pub(crate) data: Data<DerivedVariant, DeriveField>,
    pub(crate) id: Option<String>,
    pub(crate) transparent: bool,
    pub(crate) mark: bool,
}

impl DeriveInput {
    pub(crate) fn from_derive_input(input: &syn::DeriveInput) -> syn::Result<Self> {
        let ident = input.ident.clone();
        let mut id: Option<String> = None;
        let mut transparent = false;
        let mut mark = false;

        for attr in &input.attrs {
            if !attr.path().is_ident("ts") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("id") {
                    let value = meta.value()?;
                    let lit: LitStr = value.parse()?;
                    id = Some(lit.value());
                } else if meta.path.is_ident("name") {
                    let value = meta.value()?;
                    let _: LitStr = value.parse()?;
                } else if meta.path.is_ident("text") {
                    if meta.input.peek(syn::Token![=]) {
                        let value = meta.value()?;
                        let _: syn::LitBool = value.parse()?;
                    }
                } else if meta.path.is_ident("transparent") {
                    if meta.input.peek(syn::Token![=]) {
                        let value = meta.value()?;
                        let lit: syn::LitBool = value.parse()?;
                        transparent = lit.value();
                    } else {
                        transparent = true;
                    }
                } else if meta.path.is_ident("mark") {
                    if meta.input.peek(syn::Token![=]) {
                        let value = meta.value()?;
                        let lit: syn::LitBool = value.parse()?;
                        mark = lit.value();
                    } else {
                        mark = true;
                    }
                } else {
                    return Err(meta.error(format!(
                        "unknown ts attribute `{}`",
                        meta.path
                            .get_ident()
                            .map(|i| i.to_string())
                            .unwrap_or_default()
                    )));
                }
                Ok(())
            })?;
        }

        let data = match &input.data {
            syn::Data::Enum(data_enum) => {
                let mut variants = Vec::new();
                for variant in &data_enum.variants {
                    variants.push(DerivedVariant::from_variant(variant)?);
                }
                Data::Enum(variants)
            }
            syn::Data::Struct(data_struct) => {
                let fields = Fields::from_syn_fields(&data_struct.fields)?;
                Data::Struct(fields)
            }
            syn::Data::Union(_) => {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "union not supported for Parser derive",
                ));
            }
        };

        Ok(Self {
            ident,
            data,
            id,
            transparent,
            mark,
        })
    }

    /// Tree-sitter node name for this input.
    pub fn ts_node_name(&self) -> LitStr {
        let name = self
            .id
            .clone()
            .unwrap_or_else(|| self.ident.to_string().to_case(Case::Snake));
        LitStr::new(&name, self.ident.span())
    }

    /// Generate the `FromTreeSitter` implementation.
    pub fn generate(&self) -> proc_macro2::TokenStream {
        match &self.data {
            Data::Enum(fields) => self.generate_variant(fields),
            Data::Struct(fields) => self.generate_struct(fields),
        }
    }
}
