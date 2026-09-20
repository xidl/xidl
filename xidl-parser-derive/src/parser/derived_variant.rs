use super::derive_field::DeriveField;
use super::fields::Fields;
use convert_case::{Case, Casing};
use proc_macro2::Span;
use syn::LitStr;

/// Single enum variant of a Parser derive input.
#[derive(Debug)]
pub(crate) struct DerivedVariant {
    pub(crate) ident: syn::Ident,
    pub(crate) fields: Fields<DeriveField>,
    pub(crate) id: Option<String>,
}

impl DerivedVariant {
    pub(crate) fn from_variant(variant: &syn::Variant) -> syn::Result<Self> {
        let ident = variant.ident.clone();
        let mut id: Option<String> = None;

        for attr in &variant.attrs {
            if !attr.path().is_ident("ts") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("text") {
                    if meta.input.peek(syn::Token![=]) {
                        let value = meta.value()?;
                        let _: syn::LitBool = value.parse()?;
                    }
                } else if meta.path.is_ident("id") {
                    let value = meta.value()?;
                    let lit: LitStr = value.parse()?;
                    id = Some(lit.value());
                } else if meta.path.is_ident("name") {
                    // accepted for compatibility, ignored for codegen
                    let value = meta.value()?;
                    let _: LitStr = value.parse()?;
                } else if meta.path.is_ident("transparent") {
                    if meta.input.peek(syn::Token![=]) {
                        let value = meta.value()?;
                        let _: syn::LitBool = value.parse()?;
                    }
                    // variant-level transparent is not used; accept for compatibility
                } else if meta.path.is_ident("mark") {
                    if meta.input.peek(syn::Token![=]) {
                        let value = meta.value()?;
                        let _: syn::LitBool = value.parse()?;
                    }
                } else {
                    return Err(meta.error(format!(
                        "unknown ts attribute `{}` on variant",
                        meta.path
                            .get_ident()
                            .map(|i| i.to_string())
                            .unwrap_or_default()
                    )));
                }
                Ok(())
            })?;
        }

        let fields = Fields::from_syn_fields(&variant.fields)?;
        Ok(Self { ident, fields, id })
    }

    /// Tree-sitter node name for this variant.
    #[inline(always)]
    pub fn ts_node_name(&self) -> LitStr {
        let id = self.id.clone().unwrap_or_else(|| {
            // FIXME: fix field(Ty)
            self.ident.to_string().to_case(Case::Snake)
        });

        LitStr::new(&id, Span::call_site())
    }
}
