mod dds_xtypes_typeobject;

pub use dds_xtypes_typeobject::*;

pub mod runtime;

mod typeobject;
pub use typeobject::*;

extern crate self as xidl_typeobject;
