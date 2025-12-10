#![allow(dead_code)]
#![allow(unused_variables)]
use convert_case::{Case, Casing};
use darling::{FromDeriveInput, FromField, FromVariant};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Ident, LitStr};

pub fn tree_sitter_parser(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let DeriveInput {
        ident,
        data,
        kind,
        name,
        text,
    } = DeriveInput::from_derive_input(&input).unwrap();
    let ident_str = LitStr::new(
        &kind.unwrap_or_else(|| ident.to_string().to_case(Case::Snake)),
        Span::call_site(),
    );
    match data {
        darling::ast::Data::Enum(vec) => {
            let mut gen_variants = quote! {};
            for variant in vec {
                let variant_ident = &variant.ident;
                let variant_str = LitStr::new(&variant_ident.to_string().to_case(Case::Snake), Span::call_site());

                match &variant.fields.style {
                    darling::ast::Style::Unit => {
                        gen_variants.extend(quote! {
                            derive::node_id!(#variant_str) => Ok(Self::#variant_ident),
                        });
                    }
                    darling::ast::Style::Tuple => {
                        let fields = &variant.fields.fields;
                        if fields.len() == 1 {
                            let field = &fields[0];
                            gen_variants.extend(quote! {
                                derive::node_id!(#variant_str) => {
                                    Ok(Self::#variant_ident(crate::parser::FromTreeSitter::from_node(node, ctx)?))
                                }
                            });
                        }
                    }
                    darling::ast::Style::Struct => {
                    }
                }
            }
            quote! {
                impl<'a> crate::parser::FromTreeSitter<'a> for #ident {
                    fn from_node(node: tree_sitter::Node<'a>, ctx: &mut crate::parser::ParseContext<'a>) -> crate::error::ParserResult<Self> {
                        match node.kind_id() {
                            #gen_variants
                            _ => Err(crate::error::ParseError::UnexpectedNode(node.kind().to_string()))
                        }
                    }
                }
            }
        },
        darling::ast::Data::Struct(fields) => {
            let mut gen_declare = quote! {};
            let mut gen_fields = quote! {};
            let mut gen_self = quote! {};

            // struct Type;
            if fields.is_empty() {
                let text_str = LitStr::new(
                    &name.unwrap_or_else(||ident.to_string().to_case(Case::Snake)),
                    Span::call_site()
                );
                return quote! {
                        impl<'a> crate::parser::FromTreeSitter<'_> for #ident {
                            fn from_node(
                                node: tree_sitter::Node<'_>,
                                context: &mut crate::parser::ParseContext<'_>,
                            ) -> crate::error::ParserResult<Self> {
                                assert_eq!(node.utf8_text(context.source)?, #text_str);

                                Ok(Self)
                            }
                        }
                }.into();
            }

            if text {
                 let text_str = LitStr::new(
                    &name.unwrap_or_else(||ident.to_string().to_lowercase().to_string()),
                    Span::call_site()
                );
                return quote! {
                        impl<'a> crate::parser::FromTreeSitter<'_> for #ident {
                            fn from_node(
                                node: tree_sitter::Node<'_>,
                                context: &mut crate::parser::ParseContext<'_>,
                            ) -> crate::error::ParserResult<Self> {

                                Ok(Self(node.utf8_text(context.source)?.into()  ))
                            }
                        }
                }.into();
            }

            // Unit strcut

            if fields.iter().any(|item|item.ident.is_none()) {
                for (idx, field) in fields.iter().enumerate() {
                    let name = field.ident.clone().unwrap_or_else(||{
                        Ident::new(&format!("field_{idx}"), Span::call_site())
                    });

                    if field.is_vec() {
                        gen_declare.extend(quote! {
                            let mut #name = vec![];
                        });
                        gen_fields.extend(quote! {
                            if let Ok(item) = crate::parser::FromTreeSitter::from_node(ch, ctx) {
                                #name.push(item);
                            }
                        });
                    } else {
                        gen_declare.extend(quote! {
                            let mut #name = None;
                        });
                        gen_fields.extend(quote! {
                            if #name.is_none() {
                                if let Ok(item) = crate::parser::FromTreeSitter::from_node(ch, ctx) {
                                    #name = Some(item);
                                }
                            }
                        });
                    }

                    if field.is_vec() {
                        gen_self.extend(quote!{
                            #name,
                        });
                    } else {
                        gen_self.extend(quote!{
                            #name.unwrap(),
                        });
                    }
                }

                return quote! {
                    impl<'a> crate::parser::FromTreeSitter<'a> for #ident {
                        fn from_node(node: tree_sitter::Node<'a>, ctx: &mut crate::parser::ParseContext<'a>) -> crate::error::ParserResult<Self> {
                            #gen_declare
                            for ch in node.children(&mut node.walk()) {
                                #gen_fields
                            }
                            Ok(Self(
                                #gen_self
                            ))
                        }
                    }
                }.into();
            }else {




            for (idx, field) in fields.iter().enumerate() {

                let name = field.ident.clone().unwrap_or_else(||{
                    Ident::new(&format!("field_{idx}"), Span::call_site())
                });
                if field.is_span() {
                    gen_self.extend(quote! {
                        span: node.range(),
                    });
                    continue;
                }
                let name_str = LitStr::new(&name.to_string(), Span::call_site());
                gen_declare.extend(quote! {
                    let mut #name = None;
                });
                 if field.is_text() {
                    gen_fields.extend(quote! {
                        derive::node_id!(#name_str) => {
                            #name = Some(ctx.node_text(&ch)?.to_string());
                        }
                    });
                }else {
                let ty = field.ty.to_token_stream().to_string().to_case(Case::Snake);
                let ty = LitStr::new(&ty, Span::call_site() );
                    gen_fields.extend(quote!{
                        derive::node_id!(#ty) => {
                            #name = Some(crate::parser::FromTreeSitter::from_node(ch, ctx)?);
                        }
                    });
                }
                if field.is_unit() {
                        gen_self.extend(quote! {
                            #name.unwrap(),
                        });
                    }else {
                        gen_self.extend(quote! {
                            #name: #name.unwrap(),
                        });
                    }
            }
            }
            quote! {
                impl<'a> crate::parser::FromTreeSitter<'a> for #ident {
                    fn from_node(node: tree_sitter::Node<'a>, ctx: &mut crate::parser::ParseContext<'a>) -> crate::error::ParserResult<Self> {
                        assert_eq!(node.kind_id(), derive::node_id!(#ident_str));
                        #gen_declare
                        for ch in node.children(&mut node.walk()) {
                            match node.id() {
                                #gen_fields
                                _ => {}
                            }
                        }

                        Ok(Self {
                            #gen_self
                        })
                    }
                }
            }
        }
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
