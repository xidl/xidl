use crate::parser::{DeriveInput, DerivedVariant};
use quote::{quote, ToTokens};

impl DeriveInput {
    pub fn generate_variant(&self, fields: &[DerivedVariant]) -> proc_macro2::TokenStream {
        assert!(!self.mark, "variant cannot be marked");

        let ident = self.ident.clone();

        let has_unit = fields
            .iter()
            .any(|v| v.fields.style == darling::ast::Style::Unit);
        let has_tuple = fields
            .iter()
            .any(|v| v.fields.style == darling::ast::Style::Tuple);

        let using_id = !has_unit;

        let mut gen_variants = quote! {};
        for variant in fields {
            let variant_ident = &variant.ident;
            let ts_node_name = variant.ts_node_name();
            let ts_node_id = if using_id {
                quote! { xidl_derive::node_id!(#ts_node_name) }
            } else {
                quote! { #ts_node_name }
            };

            match &variant.fields.style {
                darling::ast::Style::Unit => {
                    gen_variants.extend(quote! {
                        #ts_node_id => Ok(Self::#variant_ident),
                    });
                }
                darling::ast::Style::Tuple => {
                    let fields = &variant.fields.fields;
                    if fields.len() == 1 {
                        let field = &fields[0];
                        let is_text =
                            field.text || field.ty.to_token_stream().to_string() == "String";
                        if is_text {
                            gen_variants.extend(quote! {
                                #ts_node_id => {
                                    Ok(Self::#variant_ident(ctx.node_text(&ch)?.to_string()))
                                }
                            });
                        } else {
                            gen_variants.extend(quote! {
                            #ts_node_id => {
                                Ok(Self::#variant_ident(crate::parser::FromTreeSitter::from_node(ch, ctx)?))
                            }
                        });
                        }
                    }
                }
                darling::ast::Style::Struct => {}
            }
        }

        if self.transparent {
            quote! {
                impl<'a> crate::parser::FromTreeSitter<'a> for #ident {
                    fn from_node(node: tree_sitter::Node<'a>, ctx: &mut crate::parser::ParseContext<'a>) -> crate::error::ParserResult<Self> {
                        let ch = node;
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
                        for ch in node.children(&mut node.walk()) {
                            return match ch.kind_id() {
                                #gen_variants
                                _ => Err(crate::error::ParseError::UnexpectedNode(format!("parent: {}, got: {}", node.kind(), ch.kind())))
                            }
                        }
                        unreachable!()
                    }
                }
            }
        }
    }
}
