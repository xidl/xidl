mod parser;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;

#[proc_macro_derive(Parser, attributes(ts))]
pub fn derive_parser(input: TokenStream) -> TokenStream {
    parser::tree_sitter_parser(input)
}

#[proc_macro]
pub fn node_id(input: TokenStream) -> TokenStream {
    let name = syn::parse_macro_input!(input as syn::LitStr);
    let name = name.value();

    let l = &tree_sitter_idl::language();
    let id = l.id_for_node_kind(&name, true);
    let id = id as usize;
    // if id == 0 {
    //     return syn::Error::new(Span::call_site(), format!("unknown node kind: {name}"))
    //         .into_compile_error()
    //         .into();
    // }
    let id = syn::LitInt::new(&id.to_string(), Span::call_site());

    quote! {
        #id
    }
    .into()
}

#[proc_macro]
pub fn name(input: TokenStream) -> TokenStream {
    let id = syn::parse_macro_input!(input as syn::LitInt);
    let Ok(id) = id.base10_parse() else {
        return syn::Error::new(Span::call_site(), format!("unknown field id: {id}"))
            .into_compile_error()
            .into();
    };

    let l = &tree_sitter_idl::language();
    let Some(id) = l.field_name_for_id(id) else {
        return syn::Error::new(Span::call_site(), format!("unknown field name: {id}"))
            .into_compile_error()
            .into();
    };

    quote! {
        #id
    }
    .into()
}
