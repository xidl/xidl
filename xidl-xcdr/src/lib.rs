mod c_api;
pub mod ffi;
mod formats;
pub use ffi::*;
mod types;

pub use c_api::{c_header, C_HEADER};
pub use formats::{
    Cdr3Deserializer, Cdr3Serializer, CdrDeserializer, CdrSerializer, PlcdrDeserializer,
    PlcdrSerializer,
};
pub use types::{Field, TypeDef};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SerializeError {
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, SerializeError>;

#[derive(Debug, Clone, Copy)]
pub enum Format {
    Cdr,
    Plcdr,
    Cdr3,
}

pub trait SerializeVisitor {
    fn serialize_u8(&mut self, name: &str) -> Result<()>;
    fn serialize_i8(&mut self, name: &str) -> Result<()>;
    fn serialize_u16(&mut self, name: &str) -> Result<()>;
    fn serialize_i16(&mut self, name: &str) -> Result<()>;
    fn serialize_u32(&mut self, name: &str) -> Result<()>;
    fn serialize_i32(&mut self, name: &str) -> Result<()>;
    fn serialize_u64(&mut self, name: &str) -> Result<()>;
    fn serialize_i64(&mut self, name: &str) -> Result<()>;
    fn serialize_bool(&mut self, name: &str) -> Result<()>;
    fn serialize_f32(&mut self, name: &str) -> Result<()>;
    fn serialize_f64(&mut self, name: &str) -> Result<()>;
    fn serialize_parameter_id(&mut self, id: u32) -> Result<()>;

    fn serialize_field(&mut self, field: &Field) -> Result<()>;
    fn serialize_type(&mut self, ty: &TypeDef) -> Result<()>;

    fn output(&self) -> &str;
}

pub trait DeserializeVisitor {
    fn deserialize_u8(&mut self, name: &str) -> Result<()>;
    fn deserialize_i8(&mut self, name: &str) -> Result<()>;
    fn deserialize_u16(&mut self, name: &str) -> Result<()>;
    fn deserialize_i16(&mut self, name: &str) -> Result<()>;
    fn deserialize_u32(&mut self, name: &str) -> Result<()>;
    fn deserialize_i32(&mut self, name: &str) -> Result<()>;
    fn deserialize_u64(&mut self, name: &str) -> Result<()>;
    fn deserialize_i64(&mut self, name: &str) -> Result<()>;
    fn deserialize_bool(&mut self, name: &str) -> Result<()>;
    fn deserialize_f32(&mut self, name: &str) -> Result<()>;
    fn deserialize_f64(&mut self, name: &str) -> Result<()>;

    fn deserialize_field(&mut self, field: &Field) -> Result<()>;
    fn deserialize_type(&mut self, ty: &TypeDef) -> Result<()>;

    fn output(&self) -> &str;
}

pub fn new_serializer(format: Format) -> Box<dyn SerializeVisitor> {
    match format {
        Format::Cdr => Box::new(CdrSerializer::new()),
        Format::Plcdr => Box::new(PlcdrSerializer::new()),
        Format::Cdr3 => Box::new(Cdr3Serializer::new()),
    }
}

pub fn new_deserializer(format: Format) -> Box<dyn DeserializeVisitor> {
    match format {
        Format::Cdr => Box::new(CdrDeserializer::new()),
        Format::Plcdr => Box::new(PlcdrDeserializer::new()),
        Format::Cdr3 => Box::new(Cdr3Deserializer::new()),
    }
}
