use crate::{DeserializeVisitor, Field, Result, SerializeVisitor, TypeDef};

use super::{
    deserialize_field, deserialize_type, emit, emit_parameter_id, serialize_field, serialize_type,
};

pub struct PlcdrSerializer {
    output: String,
}

pub struct PlcdrDeserializer {
    output: String,
}

impl PlcdrSerializer {
    pub fn new() -> Self {
        Self {
            output: String::new(),
        }
    }
}

impl PlcdrDeserializer {
    pub fn new() -> Self {
        Self {
            output: String::new(),
        }
    }
}

impl SerializeVisitor for PlcdrSerializer {
    fn serialize_u8(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "u8", name, "serialize");
        Ok(())
    }

    fn serialize_i8(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "i8", name, "serialize");
        Ok(())
    }

    fn serialize_u16(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "u16", name, "serialize");
        Ok(())
    }

    fn serialize_i16(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "i16", name, "serialize");
        Ok(())
    }

    fn serialize_u32(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "u32", name, "serialize");
        Ok(())
    }

    fn serialize_i32(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "i32", name, "serialize");
        Ok(())
    }

    fn serialize_u64(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "u64", name, "serialize");
        Ok(())
    }

    fn serialize_i64(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "i64", name, "serialize");
        Ok(())
    }

    fn serialize_bool(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "bool", name, "serialize");
        Ok(())
    }

    fn serialize_f32(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "f32", name, "serialize");
        Ok(())
    }

    fn serialize_f64(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "f64", name, "serialize");
        Ok(())
    }

    fn serialize_parameter_id(&mut self, id: u32) -> Result<()> {
        emit_parameter_id(&mut self.output, "plcdr", id);
        Ok(())
    }

    fn serialize_field(&mut self, field: &Field) -> Result<()> {
        serialize_field(self, field)
    }

    fn serialize_type(&mut self, ty: &TypeDef) -> Result<()> {
        serialize_type(self, ty)
    }

    fn output(&self) -> &str {
        &self.output
    }
}

impl DeserializeVisitor for PlcdrDeserializer {
    fn deserialize_u8(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "u8", name, "deserialize");
        Ok(())
    }

    fn deserialize_i8(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "i8", name, "deserialize");
        Ok(())
    }

    fn deserialize_u16(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "u16", name, "deserialize");
        Ok(())
    }

    fn deserialize_i16(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "i16", name, "deserialize");
        Ok(())
    }

    fn deserialize_u32(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "u32", name, "deserialize");
        Ok(())
    }

    fn deserialize_i32(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "i32", name, "deserialize");
        Ok(())
    }

    fn deserialize_u64(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "u64", name, "deserialize");
        Ok(())
    }

    fn deserialize_i64(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "i64", name, "deserialize");
        Ok(())
    }

    fn deserialize_bool(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "bool", name, "deserialize");
        Ok(())
    }

    fn deserialize_f32(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "f32", name, "deserialize");
        Ok(())
    }

    fn deserialize_f64(&mut self, name: &str) -> Result<()> {
        emit(&mut self.output, "plcdr", "f64", name, "deserialize");
        Ok(())
    }

    fn deserialize_field(&mut self, field: &Field) -> Result<()> {
        deserialize_field(self, field)
    }

    fn deserialize_type(&mut self, ty: &TypeDef) -> Result<()> {
        deserialize_type(self, ty)
    }

    fn output(&self) -> &str {
        &self.output
    }
}
