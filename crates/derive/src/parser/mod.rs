#![allow(dead_code)]
#![allow(unused_variables)]

mod gen_struct;
mod gen_variant;

use convert_case::{Case, Casing};
use darling::{FromDeriveInput, FromField, FromVariant};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use syn::{parse_macro_input, LitStr};

pub fn tree_sitter_parser(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let input = DeriveInput::from_derive_input(&input).unwrap();
    input.generate().into()
}

#[derive(FromDeriveInput)]
#[darling(attributes(ts), supports(any))]
struct DeriveInput {
    ident: syn::Ident,
    data: darling::ast::Data<DerivedVariant, DeriveField>,
    #[darling(default)]
    id: Option<String>,

    #[darling(default)]
    name: Option<String>,

    #[darling(default)]
    text: bool,

    /// enum
    #[darling(default)]
    transparent: bool,

    #[darling(default)]
    mark: bool,
}

impl DeriveInput {
    pub fn ts_node_name(&self) -> LitStr {
        let name = self
            .id
            .clone()
            .unwrap_or_else(|| self.ident.to_string().to_case(Case::Snake));
        LitStr::new(&name, self.ident.span())
    }

    pub fn generate(&self) -> proc_macro2::TokenStream {
        match &self.data {
            darling::ast::Data::Enum(fields) => self.generate_variant(fields),
            darling::ast::Data::Struct(fields) => self.generate_struct(fields),
        }
    }
}

#[derive(FromVariant)]
#[darling(attributes(ts))]
struct DerivedVariant {
    ident: syn::Ident,
    fields: darling::ast::Fields<DeriveField>,
    /// enum A {
    ///     #[text]
    ///     field(String) => node_id!(field) =>  Ok(Self::A(field.node_text()))
    /// }
    ///
    /// enum A {
    ///     #[text]
    ///     B => node => Ok(Self::B)
    /// }
    ///
    #[darling(default)]
    text: bool,
    #[darling(default)]
    id: Option<String>,
}

impl DerivedVariant {
    #[inline(always)]
    pub fn ts_node_name(&self) -> LitStr {
        let id = self.id.clone().unwrap_or_else(|| {
            // FIXME: fix field(Ty)
            self.ident.to_string().to_case(Case::Snake)
        });

        LitStr::new(&id, Span::call_site())
    }
}

#[derive(Debug, FromField)]
#[darling(attributes(ts))]
struct DeriveField {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    #[darling(default)]
    text: bool,
    #[darling(default)]
    id: Option<String>,
    #[darling(default)]
    transparent: bool,
}

impl DeriveField {
    #[inline(always)]
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

    pub fn is_unit(&self) -> bool {
        self.ident.is_none()
    }
    pub fn is_vec(&self) -> bool {
        match self.ty {
            syn::Type::Path(ref path) => {
                path.path.segments.len() == 1 && path.path.segments[0].ident == "Vec"
            }
            _ => false,
        }
    }

    pub fn is_option(&self) -> bool {
        match self.ty {
            syn::Type::Path(ref path) => {
                path.path.segments.len() == 1 && path.path.segments[0].ident == "Option"
            }
            _ => false,
        }
    }

    pub fn inner_ty(&self) -> syn::Type {
        if !self.is_vec() && !self.is_option() && !self.is_box() {
            return self.ty.clone();
        }
        match &self.ty {
            syn::Type::Path(ref path) => {
                let args = &path.path.segments[0].arguments;
                match args {
                    syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) => {
                        let i = &angle_bracketed_generic_arguments.args[0];
                        match i {
                            syn::GenericArgument::Type(ref ty) => ty.clone(),
                            _ => todo!(),
                        }
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }
    pub fn is_ty(&self, ty: &str) -> bool {
        match self.ty {
            syn::Type::Path(ref path) => {
                path.path.segments.len() == 1 && path.path.segments[0].ident == ty
            }
            _ => false,
        }
    }

    pub fn is_bool(&self) -> bool {
        self.is_ty("bool")
    }

    pub fn is_number(&self) -> bool {
        self.is_ty("i8")
            || self.is_ty("u8")
            || self.is_ty("i16")
            || self.is_ty("u16")
            || self.is_ty("i32")
            || self.is_ty("u32")
            || self.is_ty("i64")
            || self.is_ty("u64")
            || self.is_ty("f32")
            || self.is_ty("f64")
    }

    pub fn is_text(&self) -> bool {
        self.text || self.is_ty("String")
    }

    pub fn is_span(&self) -> bool {
        self.is_ty("Span")
    }

    pub fn is_box(&self) -> bool {
        self.is_ty("Box")
    }

    pub fn get_inner_ty(&self) -> Option<String> {
        if !self.is_box() || !self.is_vec() {
            return None;
        }
        match self.ty {
            syn::Type::Path(ref path) => Some(path.path.segments[1].ident.to_string()),
            _ => None,
        }
    }
}
