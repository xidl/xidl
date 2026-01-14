mod c_api;
// pub mod ffi;
mod formats;
// pub use ffi::*;
mod types;

pub use c_api::{C_HEADER, c_header};
pub use formats::{CdrDeserializer, CdrSerializer};
pub use types::{Field, TypeDef};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum XcdrError {
    #[error("{0}")]
    Message(String),
}

pub type XcdrResult<T> = std::result::Result<T, XcdrError>;

#[derive(Debug, Clone, Copy)]
pub enum Format {
    Cdr,
}

macro_rules! declare_ser {
    ($($id:ty)*) => {
        paste::paste!{
            $(
                fn [<serialize_ $id>](&mut self, val: $id) -> XcdrResult<()>;
            )*
        }
    }
}
pub trait SerializeVisitor {
    declare_ser! {
        u8 i8 u16 i16 u32 i32 u64 i64 bool f32 f64
    }

    fn serialize_parameter_id(&mut self, id: u32) -> XcdrResult<()>;
}

macro_rules! declare_deser {
    ($($id:ty)*) => {
        paste::paste!{
            $(
                fn [<deserialize_ $id _le>](&mut self) -> XcdrResult<$id>;
                fn [<deserialize_ $id _be>](&mut self) -> XcdrResult<$id>;
            )*
        }
    }
}

pub trait DeserializeVisitor {
    declare_deser! {
        u16 i16 u32 i32 u64 i64 f32 f64
    }

    fn deserialize_u8(&mut self) -> XcdrResult<u8>;
    fn deserialize_i8(&mut self) -> XcdrResult<i8>;
    fn deserialize_bool(&mut self) -> XcdrResult<bool>;

    fn serialize_parameter_id(&mut self) -> XcdrResult<u32>;
}

// pub fn new_serializer(format: Format) -> Box<dyn SerializeVisitor> {
//     match format {
//         Format::Cdr => Box::new(CdrSerializer::new()),
//     }
// }

// pub fn new_deserializer(format: Format) -> Box<dyn DeserializeVisitor> {
//     match format {
//         Format::Cdr => Box::new(CdrDeserializer::new()),
//     }
// }
