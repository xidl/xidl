use crate::utils::{
    FromBytes,
    align::{Align4, read_aligned},
};
use crate::{FieldId, XcdrDeserializer, error::XcdrError};

pub struct PlcdrDeserializer<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> PlcdrDeserializer<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }

    fn read_aligned<const N: usize>(&mut self) -> crate::error::XcdrResult<[u8; N]> {
        read_aligned::<Align4, N>(self.buf, &mut self.pos)
    }

    fn read_num_le<T, const N: usize>(&mut self) -> crate::error::XcdrResult<T>
    where
        T: FromBytes<N>,
    {
        Ok(T::from_le_bytes(self.read_aligned::<N>()?))
    }

    fn read_num_be<T, const N: usize>(&mut self) -> crate::error::XcdrResult<T>
    where
        T: FromBytes<N>,
    {
        Ok(T::from_be_bytes(self.read_aligned::<N>()?))
    }

    fn read_raw(&mut self, out: &mut [u8]) -> crate::error::XcdrResult<()> {
        if self.pos + out.len() > self.buf.len() {
            return Err(XcdrError::BufferOverflow);
        }
        out.copy_from_slice(&self.buf[self.pos..self.pos + out.len()]);
        self.pos += out.len();
        Ok(())
    }
}

impl XcdrDeserializer for PlcdrDeserializer<'_> {
    fn next_field(&mut self) -> crate::error::XcdrResult<Option<FieldId>> {
        if self.pos >= self.buf.len() {
            return Ok(None);
        }
        let id = u32::from_ne_bytes(self.read_aligned::<4>()?);
        Ok(Some(FieldId(id)))
    }

    fn read_u8(&mut self) -> crate::error::XcdrResult<u8> {
        self.read_num_be()
    }

    fn read_i8(&mut self) -> crate::error::XcdrResult<i8> {
        self.read_num_be()
    }

    fn read_bool(&mut self) -> crate::error::XcdrResult<bool> {
        self.read_num_be()
    }

    fn read_u16_le(&mut self) -> crate::error::XcdrResult<u16> {
        self.read_num_le()
    }

    fn read_u16_be(&mut self) -> crate::error::XcdrResult<u16> {
        self.read_num_be()
    }

    fn read_i16_le(&mut self) -> crate::error::XcdrResult<i16> {
        self.read_num_le()
    }

    fn read_i16_be(&mut self) -> crate::error::XcdrResult<i16> {
        self.read_num_be()
    }

    fn read_u32_le(&mut self) -> crate::error::XcdrResult<u32> {
        self.read_num_le()
    }

    fn read_u32_be(&mut self) -> crate::error::XcdrResult<u32> {
        self.read_num_be()
    }

    fn read_i32_le(&mut self) -> crate::error::XcdrResult<i32> {
        self.read_num_le()
    }

    fn read_i32_be(&mut self) -> crate::error::XcdrResult<i32> {
        self.read_num_be()
    }

    fn read_u64_le(&mut self) -> crate::error::XcdrResult<u64> {
        self.read_num_le()
    }

    fn read_u64_be(&mut self) -> crate::error::XcdrResult<u64> {
        self.read_num_be()
    }

    fn read_i64_le(&mut self) -> crate::error::XcdrResult<i64> {
        self.read_num_le()
    }

    fn read_i64_be(&mut self) -> crate::error::XcdrResult<i64> {
        self.read_num_be()
    }

    fn read_f32_le(&mut self) -> crate::error::XcdrResult<f32> {
        self.read_num_le()
    }

    fn read_f32_be(&mut self) -> crate::error::XcdrResult<f32> {
        self.read_num_be()
    }

    fn read_f64_le(&mut self) -> crate::error::XcdrResult<f64> {
        self.read_num_le()
    }

    fn read_f64_be(&mut self) -> crate::error::XcdrResult<f64> {
        self.read_num_be()
    }

    fn read_bytes(&mut self, out: &mut [u8]) -> crate::error::XcdrResult<()> {
        self.read_raw(out)
    }
}
