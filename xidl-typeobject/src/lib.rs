mod dds_xtypes_typeobject;
use core::mem::ManuallyDrop;

pub use dds_xtypes_typeobject::*;

pub mod runtime;

mod typeobject;
pub use typeobject::*;

extern crate self as xidl_typeobject;

union X {
    v: ManuallyDrop<u32>,
}

impl Drop for X {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.v);
        }
    }
}
