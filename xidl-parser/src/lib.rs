pub mod error;
pub mod parser;
pub mod semantic;
pub mod typed_ast;

pub mod hir;
pub mod jsonrpc_hir;
pub mod rest_hir;
pub mod versioning;

pub use xidl_parser_derive::Parser;

#[cfg(test)]
mod tests;
