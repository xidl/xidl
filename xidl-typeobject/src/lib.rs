mod dds_xtypes_typeobject;
pub mod runtime;

pub use dds_xtypes_typeobject::*;

extern crate self as xidl_typeobject;

pub trait XidlTypeObject {
    fn minimal_type_object() -> DDS::XTypes::TypeObject;
    fn complete_type_object() -> DDS::XTypes::TypeObject;
}
