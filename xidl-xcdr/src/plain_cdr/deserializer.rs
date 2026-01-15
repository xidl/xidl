use crate::utils::FromBytes;
use crate::{error::XcdrError, XcdrDeserialize};

pub struct PlainCdrDeserializer<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> PlainCdrDeserializer<'a> {
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
        let align = match N % 8 {
            0 => 0usize,
            v => 8 - v,
        };
        if self.pos + align + N > self.buf.len() {
            return Err(XcdrError::BufferOverflow);
        }

        self.pos += align;
        let mut out = [0u8; N];
        out.copy_from_slice(&self.buf[self.pos..self.pos + N]);
        self.pos += N;
        Ok(out)
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

impl XcdrDeserialize for PlainCdrDeserializer<'_> {
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
