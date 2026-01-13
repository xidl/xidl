use super::*;
use xidl_derive::Parser;

#[derive(Debug, Parser)]
pub struct ExceptDcl {
    pub ident: Identifier,
    pub member: Vec<Member>,
}
