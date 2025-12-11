use crate::parser::{DeriveInput, DerivedVariant};
use convert_case::{Case, Casing};
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::LitStr;

pub fn generate_variant(input: &DeriveInput, vec: &[DerivedVariant]) -> proc_macro2::TokenStream {
    let mut gen_variants = quote! {};
    for variant in vec {
        let variant_ident = &variant.ident;
        let variant_str = LitStr::new(
            &variant_ident.to_string().to_case(Case::Snake),
            Span::call_site(),
        );

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
                    let is_text = field.text || field.ty.to_token_stream().to_string() == "String";
                    if is_text {
                        gen_variants.extend(quote! {
                            derive::node_id!(#variant_str) => {
                                Ok(Self::#variant_ident(ctx.node_text(&node)?.to_string()))
                            }
                        });
                    } else {
                        gen_variants.extend(quote! {
                            derive::node_id!(#variant_str) => {
                                Ok(Self::#variant_ident(crate::parser::FromTreeSitter::from_node(node, ctx)?))
                            }
                        });
                    }
                }
            }
            darling::ast::Style::Struct => {}
        }
    }

    let ident = input.ident.clone();

    if input.transparent {
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
    } else {
        quote! {
            impl<'a> crate::parser::FromTreeSitter<'a> for #ident {
                fn from_node(node: tree_sitter::Node<'a>, ctx: &mut crate::parser::ParseContext<'a>) -> crate::error::ParserResult<Self> {
                    for node in node.children(&mut node.walk()) {
                        return match node.kind_id() {
                            #gen_variants
                            _ => Err(crate::error::ParseError::UnexpectedNode(node.kind().to_string()))
                        }
                    }
                    unreachable!()
                }
            }
        }
    }
}
