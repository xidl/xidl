#![allow(dead_code)]
#![allow(unused_variables)]

mod gen_struct;
mod gen_variant;

use convert_case::{Case, Casing};
use darling::{FromDeriveInput, FromField, FromVariant};
use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{parse_macro_input, LitStr};

use crate::parser::gen_variant::generate_variant;

pub fn tree_sitter_parser(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let input = DeriveInput::from_derive_input(&input).unwrap();
    let ident_str = LitStr::new(
        &input
            .kind
            .clone()
            .unwrap_or_else(|| input.ident.to_string().to_case(Case::Snake)),
        Span::call_site(),
    );
    match &input.data {
        darling::ast::Data::Enum(vec) => generate_variant(&input, vec),
        darling::ast::Data::Struct(fields) => gen_struct::generate_variant(&input, fields),
    }
    .into()
}

#[derive(FromDeriveInput)]
#[darling(attributes(ts), supports(any))]
struct DeriveInput {
    ident: syn::Ident,
    data: darling::ast::Data<DerivedVariant, DeriveField>,
    #[darling(default)]
    kind: Option<String>,

    #[darling(default)]
    name: Option<String>,

    #[darling(default)]
    text: bool,

    /// enum
    #[darling(default)]
    transparent: bool,
}

#[derive(FromVariant)]
#[darling(attributes(ts))]
struct DerivedVariant {
    ident: syn::Ident,
    fields: darling::ast::Fields<DeriveField>,
    #[darling(default)]
    text: bool,
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
}

impl DeriveField {
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

fn query_id_by_name(name: &str) -> Option<u16> {
    let ts = &tree_sitter_idl::language();
    Some(ts.field_id_for_name(name)?.get())
}
