pub mod error;

pub mod cdr;

mod utils;

use error::XcdrResult;

pub struct FieldId(pub u32);

#[allow(unused_variables)]
pub trait XcdrSerialize {
    fn begin_struct(&mut self) -> XcdrResult<()> {
        Ok(())
    }
    fn end_struct(&mut self) -> XcdrResult<()> {
        Ok(())
    }

    fn begin_field(&mut self, id: FieldId) -> XcdrResult<()> {
        Ok(())
    }
    fn end_field(&mut self) -> XcdrResult<()> {
        Ok(())
    }

    fn write_bool(&mut self, val: bool) -> XcdrResult<()>;
    fn write_u8(&mut self, val: u8) -> XcdrResult<()>;
    fn write_i8(&mut self, val: i8) -> XcdrResult<()>;
    fn write_u16(&mut self, val: u16) -> XcdrResult<()>;
    fn write_i16(&mut self, val: i16) -> XcdrResult<()>;
    fn write_u32(&mut self, val: u32) -> XcdrResult<()>;
    fn write_i32(&mut self, val: i32) -> XcdrResult<()>;
    fn write_u64(&mut self, val: u64) -> XcdrResult<()>;
    fn write_i64(&mut self, val: i64) -> XcdrResult<()>;
    fn write_f32(&mut self, val: f32) -> XcdrResult<()>;
    fn write_f64(&mut self, val: f64) -> XcdrResult<()>;
    fn write_bytes(&mut self, buf: &[u8]) -> XcdrResult<()>;
}

pub trait XcdrDeserialize {
    fn next_field(&mut self) -> XcdrResult<Option<FieldId>> {
        Ok(None)
    }
    fn enter_struct(&mut self) -> XcdrResult<()> {
        Ok(())
    }
    fn exit_struct(&mut self) -> XcdrResult<()> {
        Ok(())
    }
    fn skip_field(&mut self) -> XcdrResult<()> {
        Ok(())
    }

    fn read_u8(&mut self) -> XcdrResult<u8>;
    fn read_i8(&mut self) -> XcdrResult<i8>;
    fn read_bool(&mut self) -> XcdrResult<bool>;
    fn read_u16_le(&mut self) -> XcdrResult<u16>;
    fn read_u16_be(&mut self) -> XcdrResult<u16>;
    fn read_i16_le(&mut self) -> XcdrResult<i16>;
    fn read_i16_be(&mut self) -> XcdrResult<i16>;
    fn read_u32_le(&mut self) -> XcdrResult<u32>;
    fn read_u32_be(&mut self) -> XcdrResult<u32>;
    fn read_i32_le(&mut self) -> XcdrResult<i32>;
    fn read_i32_be(&mut self) -> XcdrResult<i32>;
    fn read_u64_le(&mut self) -> XcdrResult<u64>;
    fn read_u64_be(&mut self) -> XcdrResult<u64>;
    fn read_i64_le(&mut self) -> XcdrResult<i64>;
    fn read_i64_be(&mut self) -> XcdrResult<i64>;
    fn read_f32_le(&mut self) -> XcdrResult<f32>;
    fn read_f32_be(&mut self) -> XcdrResult<f32>;
    fn read_f64_le(&mut self) -> XcdrResult<f64>;
    fn read_f64_be(&mut self) -> XcdrResult<f64>;
    fn read_bytes(&mut self, out: &mut [u8]) -> XcdrResult<()>;
}
