use super::*;
use serde::{Deserialize, Serialize};
use xidl_parser_derive::Parser;

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct ExceptDcl {
    pub ident: Identifier,
    pub member: Vec<Member>,
}
