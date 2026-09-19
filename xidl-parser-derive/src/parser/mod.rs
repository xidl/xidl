#![allow(dead_code)]
#![allow(unused_variables)]

mod gen_struct;
mod gen_variant;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use syn::{LitStr, parse_macro_input};

pub fn tree_sitter_parser(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let derived = match DeriveInput::from_derive_input(&input) {
        Ok(v) => v,
        Err(err) => return err.to_compile_error().into(),
    };
    derived.generate().into()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Style {
    Unit,
    Tuple,
    Struct,
}

#[derive(Debug)]
pub struct Fields<T> {
    pub style: Style,
    pub fields: Vec<T>,
}

impl<T> Fields<T> {
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.fields.iter()
    }
}

#[derive(Debug)]
pub enum Data<V, F> {
    Enum(Vec<V>),
    Struct(Fields<F>),
}

#[derive(Debug)]
struct DeriveInput {
    ident: syn::Ident,
    data: Data<DerivedVariant, DeriveField>,
    id: Option<String>,
    name: Option<String>,
    text: bool,
    transparent: bool,
    mark: bool,
}

impl DeriveInput {
    fn from_derive_input(input: &syn::DeriveInput) -> syn::Result<Self> {
        let ident = input.ident.clone();
        let mut id: Option<String> = None;
        let mut name: Option<String> = None;
        let mut text = false;
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
                    let lit: LitStr = value.parse()?;
                    name = Some(lit.value());
                } else if meta.path.is_ident("text") {
                    if meta.input.peek(syn::Token![=]) {
                        let value = meta.value()?;
                        let lit: syn::LitBool = value.parse()?;
                        text = lit.value();
                    } else {
                        text = true;
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
            name,
            text,
            transparent,
            mark,
        })
    }

    pub fn ts_node_name(&self) -> LitStr {
        let name = self
            .id
            .clone()
            .unwrap_or_else(|| self.ident.to_string().to_case(Case::Snake));
        LitStr::new(&name, self.ident.span())
    }

    pub fn generate(&self) -> proc_macro2::TokenStream {
        match &self.data {
            Data::Enum(fields) => self.generate_variant(fields),
            Data::Struct(fields) => self.generate_struct(fields),
        }
    }
}

#[derive(Debug)]
struct DerivedVariant {
    ident: syn::Ident,
    fields: Fields<DeriveField>,
    text: bool,
    id: Option<String>,
}

impl DerivedVariant {
    fn from_variant(variant: &syn::Variant) -> syn::Result<Self> {
        let ident = variant.ident.clone();
        let mut text = false;
        let mut id: Option<String> = None;

        for attr in &variant.attrs {
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
                } else if meta.path.is_ident("name") {
                    // accepted for compatibility, ignored for codegen (like DeriveInput::name)
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
        Ok(Self {
            ident,
            fields,
            text,
            id,
        })
    }

    #[inline(always)]
    pub fn ts_node_name(&self) -> LitStr {
        let id = self.id.clone().unwrap_or_else(|| {
            // FIXME: fix field(Ty)
            self.ident.to_string().to_case(Case::Snake)
        });

        LitStr::new(&id, Span::call_site())
    }
}

#[derive(Debug)]
struct DeriveField {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    text: bool,
    id: Option<String>,
    transparent: bool,
}

impl DeriveField {
    fn from_field(field: &syn::Field) -> syn::Result<Self> {
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

impl<T> Fields<T>
where
    T: FromSynField,
{
    fn from_syn_fields(fields: &syn::Fields) -> syn::Result<Self> {
        match fields {
            syn::Fields::Unit => Ok(Self {
                style: Style::Unit,
                fields: Vec::new(),
            }),
            syn::Fields::Named(named) => {
                let mut vec = Vec::new();
                for field in &named.named {
                    vec.push(T::from_syn_field(field)?);
                }
                Ok(Self {
                    style: Style::Struct,
                    fields: vec,
                })
            }
            syn::Fields::Unnamed(unnamed) => {
                let mut vec = Vec::new();
                for field in &unnamed.unnamed {
                    vec.push(T::from_syn_field(field)?);
                }
                Ok(Self {
                    style: Style::Tuple,
                    fields: vec,
                })
            }
        }
    }
}

pub(crate) trait FromSynField: Sized {
    fn from_syn_field(field: &syn::Field) -> syn::Result<Self>;
}

impl FromSynField for DeriveField {
    fn from_syn_field(field: &syn::Field) -> syn::Result<Self> {
        Self::from_field(field)
    }
}
