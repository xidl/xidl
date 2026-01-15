use super::*;
use xidl_derive::Parser;

#[derive(Debug, Parser)]
pub struct ModuleDcl {
    pub annotations: Vec<AnnotationAppl>,
    pub ident: Identifier,
    pub definition: Vec<Definition>,
}
