use crate::parser::{DeriveField, DeriveInput};
use convert_case::{Case, Casing};
use darling::ast::Fields;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{Ident, LitStr};

pub fn generate_variant(
    input: &DeriveInput,
    fields: &Fields<DeriveField>,
) -> proc_macro2::TokenStream {
    // if fields.transparent | count > 1 => panic
    // if fields.transparent | count > 1 && fields.last.transparent == false => panic

    let debug = input.ident == "EnumDcl";
    let mut gen_declare = quote! {};
    let mut gen_fields = quote! {};
    let mut gen_self = quote! {};
    let ident = input.ident.clone();
    let ident_str = LitStr::new(
        &input
            .kind
            .clone()
            .unwrap_or_else(|| ident.to_string().to_case(Case::Snake)),
        Span::call_site(),
    );

    // struct Type;
    if fields.is_empty() {
        let text_str = LitStr::new(
            &input
                .name
                .clone()
                .unwrap_or_else(|| ident.to_string().to_case(Case::Snake)),
            Span::call_site(),
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
        };
    }

    if input.text {
        let text_str = LitStr::new(
            &input
                .name
                .clone()
                .unwrap_or_else(|| ident.to_string().to_lowercase().to_string()),
            Span::call_site(),
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
        };
    }

    // Unit strcut
    let is_unit_struct = fields.iter().any(|item| item.ident.is_none());
    if is_unit_struct {
        for (idx, field) in fields.iter().enumerate() {
            let name = field
                .ident
                .clone()
                .unwrap_or_else(|| Ident::new(&format!("field_{idx}"), Span::call_site()));

            let ts_node_name = field.ts_node_name();
            let condition = if field.transparent {
                quote! {
                    _
                }
            } else {
                quote! {
                    derive::node_id!(#ts_node_name)
                }
            };

            if field.is_vec() {
                gen_declare.extend(quote! {
                    let mut #name = vec![];
                });
                gen_fields.extend(quote! {
                    #condition => {
                        #name.push(crate::parser::FromTreeSitter::from_node(ch, ctx)?);
                    }
                });
            } else {
                gen_declare.extend(quote! {
                    let mut #name = None;
                });
                gen_fields.extend(quote! {
                    #condition => {
                        #name = Some(crate::parser::FromTreeSitter::from_node(ch, ctx)?);
                    }
                });
            }

            if field.is_vec() {
                gen_self.extend(quote! {
                    #name,
                });
            } else {
                gen_self.extend(quote! {
                    #name.unwrap(),
                });
            }
        }

        let has_transparent = fields.iter().any(|f| f.transparent);
        let default_branch = if has_transparent {
            quote! {}
        } else {
            quote! {
                _ => {}
            }
        };

        return quote! {
            impl<'a> crate::parser::FromTreeSitter<'a> for #ident {
                fn from_node(node: tree_sitter::Node<'a>, ctx: &mut crate::parser::ParseContext<'a>) -> crate::error::ParserResult<Self> {
                    #gen_declare
                    for ch in node.children(&mut node.walk()) {
                        match ch.kind_id() {
                            #gen_fields
                            #default_branch
                        }
                    }
                    Ok(Self(
                        #gen_self
                    ))
                }
            }
        };
    } else {
        for (idx, field) in fields.iter().enumerate() {
            let name = field
                .ident
                .clone()
                .unwrap_or_else(|| Ident::new(&format!("field_{idx}"), Span::call_site()));
            if field.is_span() {
                gen_self.extend(quote! {
                    span: node.range(),
                });
                continue;
            }

            let mut name_str = LitStr::new(&name.to_string(), Span::call_site());
            if field.is_vec() {
                gen_declare.extend(quote! {
                    let mut #name = vec![];
                });
                name_str = LitStr::new(
                    &field
                        .inner_ty()
                        .to_token_stream()
                        .to_string()
                        .to_case(Case::Snake),
                    Span::call_site(),
                );
            } else {
                gen_declare.extend(quote! {
                    let mut #name = None;
                });
            }
            if field.is_text() {
                gen_fields.extend(quote! {
                    derive::node_id!(#name_str) => {
                        #name = Some(ctx.node_text(&ch)?.to_string());
                    }
                });
            } else {
                let ty = field.ty.to_token_stream().to_string().to_case(Case::Snake);
                let ty = LitStr::new(&ty, Span::call_site());

                if field.is_vec() {
                    gen_fields.extend(quote! {
                        derive::node_id!(#name_str) => {
                            #name.push(crate::parser::FromTreeSitter::from_node(ch, ctx)?);
                        }
                    });
                } else {
                    gen_fields.extend(quote! {
                        derive::node_id!(#ty) => {
                            #name = Some(crate::parser::FromTreeSitter::from_node(ch, ctx)?);
                        }
                    });
                }
            }

            if field.is_vec() || field.is_option() {
                gen_self.extend(quote! {
                    #name,
                });
                continue;
            }
            if field.is_unit() {
                gen_self.extend(quote! {
                    #name.unwrap(),
                });
            } else {
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
                    match ch.kind_id() {
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
