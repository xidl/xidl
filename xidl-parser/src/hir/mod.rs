mod annotation;
mod annotation_builtin;
mod compound;
mod const_dcl;
mod declarator;
mod enum_dcl;
mod exception_dcl;
mod expr;
mod include;
mod interface;
mod interface_codegen;
mod pragma;
mod scoped_name;
mod spec;
mod struct_dcl;
mod type_dcl;
mod types;

pub use annotation::*;
pub use compound::*;
pub use const_dcl::*;
pub use declarator::*;
pub use enum_dcl::*;
pub use exception_dcl::*;
pub use expr::*;
pub use include::*;
pub use interface::*;
pub use pragma::*;
pub use scoped_name::*;
pub use struct_dcl::*;
pub use type_dcl::*;
pub use types::*;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Specification(pub Vec<Definition>);

pub type ParserProperties = HashMap<String, Value>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Definition {
    ModuleDcl(ModuleDcl),
    Pragma(Pragma),
    ConstrTypeDcl(ConstrTypeDcl),
    TypeDcl(TypeDcl),
    ConstDcl(ConstDcl),
    ExceptDcl(ExceptDcl),
    InterfaceDcl(InterfaceDcl),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModuleDcl {
    pub annotations: Vec<Annotation>,
    pub ident: String,
    pub definition: Vec<Definition>,
}
