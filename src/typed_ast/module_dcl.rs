use super::*;
use derive::Parser;

#[derive(Debug, Parser)]
pub struct ModuleDcl {
    pub ident: Identifier,
    pub definition: Vec<Definition>,
}
