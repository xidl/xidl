mod cdr;
mod cdr3;
mod plcdr;

pub use cdr::{CdrDeserializer, CdrSerializer};
pub use cdr3::{Cdr3Deserializer, Cdr3Serializer};
pub use plcdr::{PlcdrDeserializer, PlcdrSerializer};

use crate::{DeserializeVisitor, Field, Result, SerializeError, SerializeVisitor, TypeDef};

fn serialize_type<V: SerializeVisitor>(visitor: &mut V, ty: &TypeDef) -> Result<()> {
    for field in &ty.fields {
        visitor.serialize_field(field)?;
    }
    Ok(())
}

fn serialize_field<V: SerializeVisitor>(visitor: &mut V, field: &Field) -> Result<()> {
    match field.ty.as_str() {
        "u8" => visitor.serialize_u8(&field.name),
        "i8" => visitor.serialize_i8(&field.name),
        "u16" => visitor.serialize_u16(&field.name),
        "i16" => visitor.serialize_i16(&field.name),
        "u32" => visitor.serialize_u32(&field.name),
        "i32" => visitor.serialize_i32(&field.name),
        "u64" => visitor.serialize_u64(&field.name),
        "i64" => visitor.serialize_i64(&field.name),
        "bool" => visitor.serialize_bool(&field.name),
        "f32" => visitor.serialize_f32(&field.name),
        "f64" => visitor.serialize_f64(&field.name),
        other => Err(SerializeError::Message(format!(
            "unsupported field type: {other}"
        ))),
    }
}

fn deserialize_type<V: DeserializeVisitor>(visitor: &mut V, ty: &TypeDef) -> Result<()> {
    for field in &ty.fields {
        visitor.deserialize_field(field)?;
    }
    Ok(())
}

fn deserialize_field<V: DeserializeVisitor>(visitor: &mut V, field: &Field) -> Result<()> {
    match field.ty.as_str() {
        "u8" => visitor.deserialize_u8(&field.name),
        "i8" => visitor.deserialize_i8(&field.name),
        "u16" => visitor.deserialize_u16(&field.name),
        "i16" => visitor.deserialize_i16(&field.name),
        "u32" => visitor.deserialize_u32(&field.name),
        "i32" => visitor.deserialize_i32(&field.name),
        "u64" => visitor.deserialize_u64(&field.name),
        "i64" => visitor.deserialize_i64(&field.name),
        "bool" => visitor.deserialize_bool(&field.name),
        "f32" => visitor.deserialize_f32(&field.name),
        "f64" => visitor.deserialize_f64(&field.name),
        other => Err(SerializeError::Message(format!(
            "unsupported field type: {other}"
        ))),
    }
}

fn emit(buf: &mut String, format: &str, ty: &str, name: &str, prefix: &str) {
    if !buf.is_empty() {
        buf.push('\n');
    }
    buf.push_str(&format!("{format}::{prefix}_{ty}({name});"));
}

fn emit_parameter_id(buf: &mut String, format: &str, id: u32) {
    if !buf.is_empty() {
        buf.push('\n');
    }
    buf.push_str(&format!("{format}::serialize_parameter_id({id});"));
}
