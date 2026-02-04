use crate::parser::{DeriveField, DeriveInput};
use darling::ast::Fields;
use proc_macro2::Span;
use quote::quote;
use syn::Ident;

impl DeriveInput {
    fn generate_mark(&self, fields: &Fields<DeriveField>) -> proc_macro2::TokenStream {
        let ident = self.ident.clone();
        let ts_node_name = self.ts_node_name();

        quote! {
            impl<'a> crate::parser::FromTreeSitter<'a> for #ident {
                fn from_node(node: tree_sitter::Node<'a>, ctx: &mut crate::parser::ParseContext<'a>) -> crate::error::ParserResult<Self> {
                    debug_assert_eq!(node.kind_id(), xidl_parser_derive::node_id!(#ts_node_name));
                    Ok(Self)
                }
            }
        }
    }

    fn generate_unit_transparent(&self, fields: &Fields<DeriveField>) -> proc_macro2::TokenStream {
        let ident = self.ident.clone();
        let ts_node_name = self.ts_node_name();

        quote! {
            impl<'a> crate::parser::FromTreeSitter<'a> for #ident {
                fn from_node(node: tree_sitter::Node<'a>, ctx: &mut crate::parser::ParseContext<'a>) -> crate::error::ParserResult<Self> {
                    debug_assert_eq!(node.kind_id(), xidl_parser_derive::node_id!(#ts_node_name));
                    Ok(Self(crate::parser::FromTreeSitter::from_node(node, ctx)?))
                }
            }
        }
    }
    /// #[ts(transparent)]
    /// pub struct Identifier(String);
    ///   (identifier) ; [0, 11] - [0, 12]
    fn is_generate_unit_transparent(&self) -> bool {
        match &self.data {
            darling::ast::Data::Struct(f) => f.len() == 1 && self.transparent,
            _ => false,
        }
    }

    pub fn is_mark(&self) -> bool {
        let is_marked_type = match &self.data {
            darling::ast::Data::Enum(_) => false,
            darling::ast::Data::Struct(fields) => fields.is_empty(),
        };
        is_marked_type || self.mark
    }

    pub fn generate_struct(&self, fields: &Fields<DeriveField>) -> proc_macro2::TokenStream {
        // if fields.transparent | count > 1 => panic
        // if fields.transparent | count > 1 && fields.last.transparent == false => panic
        let ident = self.ident.clone();
        // struct M {};
        // or
        // #[ts(mark)]
        // struct N;
        if self.is_mark() {
            return self.generate_mark(fields);
        }

        if self.is_generate_unit_transparent() {
            return self.generate_unit_transparent(fields);
        }

        let mut gen_declare = quote! {};
        let mut gen_fields = quote! {};
        let mut gen_self = quote! {};
        let ident = self.ident.clone();
        let ts_node_name = self.ts_node_name();

        // Tuple strcut
        // struct A(i32, i32);
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
                        xidl_parser_derive::node_id!(#ts_node_name)
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

                let ts_node_name = field.ts_node_name();

                // Special handling for fields with id = "-" (node-level text capture)
                if ts_node_name.value() == "-" && field.is_text() {
                    gen_declare.extend(quote! {
                        let #name = ctx.node_text(&node)?.to_string();
                    });
                    gen_self.extend(quote! {
                        #name,
                    });
                    continue;
                }

                if field.is_vec() {
                    gen_declare.extend(quote! {
                        let mut #name = vec![];
                    });
                } else {
                    gen_declare.extend(quote! {
                        let mut #name = None;
                    });
                }
                if field.is_text() {
                    gen_fields.extend(quote! {
                        xidl_parser_derive::node_id!(#ts_node_name) => {
                            #name = Some(ctx.node_text(&ch)?.to_string());
                        }
                    });
                } else {
                    #[allow(clippy::collapsible_else_if)]
                    if field.is_vec() {
                        gen_fields.extend(quote! {
                            xidl_parser_derive::node_id!(#ts_node_name) => {
                                #name.push(crate::parser::FromTreeSitter::from_node(ch, ctx)?);
                            }
                        });
                    } else {
                        gen_fields.extend(quote! {
                            xidl_parser_derive::node_id!(#ts_node_name) => {
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
                    assert_eq!(node.kind_id(), xidl_parser_derive::node_id!(#ts_node_name));
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
}
