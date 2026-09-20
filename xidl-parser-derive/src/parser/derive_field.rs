use super::fields::FromSynField;
use convert_case::{Case, Casing};
use proc_macro2::Span;
use quote::ToTokens;
use syn::LitStr;

/// Single field of a Parser derive input.
#[derive(Debug)]
pub(crate) struct DeriveField {
    pub(crate) ident: Option<syn::Ident>,
    pub(crate) ty: syn::Type,
    pub(crate) text: bool,
    pub(crate) id: Option<String>,
    pub(crate) transparent: bool,
}

impl DeriveField {
    pub(crate) fn from_field(field: &syn::Field) -> syn::Result<Self> {
        let ident = field.ident.clone();
        let ty = field.ty.clone();
        let mut text = false;
        let mut id: Option<String> = None;
        let mut transparent = false;

        for attr in &field.attrs {
            if !attr.path().is_ident("ts") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("text") {
                    if meta.input.peek(syn::Token![=]) {
                        let value = meta.value()?;
                        let lit: syn::LitBool = value.parse()?;
                        text = lit.value();
                    } else {
                        text = true;
                    }
                } else if meta.path.is_ident("id") {
                    let value = meta.value()?;
                    let lit: LitStr = value.parse()?;
                    id = Some(lit.value());
                } else if meta.path.is_ident("transparent") {
                    if meta.input.peek(syn::Token![=]) {
                        let value = meta.value()?;
                        let lit: syn::LitBool = value.parse()?;
                        transparent = lit.value();
                    } else {
                        transparent = true;
                    }
                } else if meta.path.is_ident("name") {
                    let value = meta.value()?;
                    let _: LitStr = value.parse()?;
                } else if meta.path.is_ident("mark") {
                    if meta.input.peek(syn::Token![=]) {
                        let value = meta.value()?;
                        let _: syn::LitBool = value.parse()?;
                    }
                } else {
                    return Err(meta.error(format!(
                        "unknown ts attribute `{}` on field",
                        meta.path
                            .get_ident()
                            .map(|i| i.to_string())
                            .unwrap_or_default()
                    )));
                }
                Ok(())
            })?;
        }

        Ok(Self {
            ident,
            ty,
            text,
            id,
            transparent,
        })
    }

    /// Tree-sitter node name for this field.
    pub fn ts_node_name(&self) -> LitStr {
        let mut id = self.id.clone().unwrap_or_else(|| {
            self.inner_ty()
                .to_token_stream()
                .to_string()
                .to_case(Case::Snake)
        });
        if self.transparent {
            id = "-".to_string();
        }
        LitStr::new(&id, Span::call_site())
    }

    /// Whether the field is an unnamed tuple position.
    pub fn is_unit(&self) -> bool {
        self.ident.is_none()
    }

    /// Whether the field type is exactly `Vec`.
    pub fn is_vec(&self) -> bool {
        match self.ty {
            syn::Type::Path(ref path) => {
                path.path.segments.len() == 1 && path.path.segments[0].ident == "Vec"
            }
            _ => false,
        }
    }

    /// Whether the field type is exactly `Option`.
    pub fn is_option(&self) -> bool {
        match self.ty {
            syn::Type::Path(ref path) => {
                path.path.segments.len() == 1 && path.path.segments[0].ident == "Option"
            }
            _ => false,
        }
    }

    /// Inner type of a `Vec`, `Option`, or `Box` wrapper.
    pub fn inner_ty(&self) -> syn::Type {
        if !self.is_vec() && !self.is_option() && !self.is_box() {
            return self.ty.clone();
        }
        match &self.ty {
            syn::Type::Path(path) => {
                let args = &path.path.segments[0].arguments;
                match args {
                    syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) => {
                        let i = &angle_bracketed_generic_arguments.args[0];
                        match i {
                            syn::GenericArgument::Type(ty) => ty.clone(),
                            _ => todo!(),
                        }
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }

    /// Whether the field type is exactly the named type.
    pub fn is_ty(&self, ty: &str) -> bool {
        match self.ty {
            syn::Type::Path(ref path) => {
                path.path.segments.len() == 1 && path.path.segments[0].ident == ty
            }
            _ => false,
        }
    }

    /// Whether the field captures node text.
    pub fn is_text(&self) -> bool {
        self.text || self.is_ty("String")
    }

    /// Whether the field type is exactly `Span`.
    pub fn is_span(&self) -> bool {
        self.is_ty("Span")
    }

    /// Whether the field type is exactly `Box`.
    pub fn is_box(&self) -> bool {
        self.is_ty("Box")
    }
}

impl FromSynField for DeriveField {
    fn from_syn_field(field: &syn::Field) -> syn::Result<Self> {
        Self::from_field(field)
    }
}
