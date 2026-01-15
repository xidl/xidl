use crate::utils::FromBytes;
use crate::{error::XcdrError, XcdrDeserialize};

const ENDIAN_FLAG: u32 = 1 << 31;
const LEN_MASK: u32 = !ENDIAN_FLAG;

pub struct DelimitedCdrDeserializer<'a> {
    buf: &'a [u8],
    pos: usize,
    end_pos: Option<usize>,
    pub header_little_endian: bool,
}

impl<'a> DelimitedCdrDeserializer<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self {
            buf,
            pos: 0,
            end_pos: None,
            header_little_endian: cfg!(target_endian = "little"),
        }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub fn end_position(&self) -> Option<usize> {
        self.end_pos
    }

    pub fn set_end_position(&mut self, end_pos: Option<usize>) {
        self.end_pos = end_pos;
    }

    fn limit(&self) -> usize {
        self.end_pos.unwrap_or(self.buf.len())
    }

    fn align_for_len(&mut self, len: usize) -> crate::error::XcdrResult<()> {
        let align_to = match len {
            8 | 16 => 4usize,
            _ => 8usize,
        };
        let align = match len % align_to {
            0 => 0usize,
            v => align_to - v,
        };
        let limit = self.limit();
        if self.pos + align > limit {
            return Err(XcdrError::BufferOverflow);
        }
        self.pos += align;
        Ok(())
    }

    fn read_aligned<const N: usize>(&mut self) -> crate::error::XcdrResult<[u8; N]> {
        let len = N;
        self.align_for_len(len)?;
        let limit = self.limit();
        if self.pos + len > limit {
            return Err(XcdrError::BufferOverflow);
        }

        let mut out = [0u8; N];
        out.copy_from_slice(&self.buf[self.pos..self.pos + len]);
        self.pos += len;
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
        let limit = self.limit();
        if self.pos + out.len() > limit {
            return Err(XcdrError::BufferOverflow);
        }
        out.copy_from_slice(&self.buf[self.pos..self.pos + out.len()]);
        self.pos += out.len();
        Ok(())
    }

    fn read_u32_ne(&mut self) -> crate::error::XcdrResult<u32> {
        Ok(u32::from_ne_bytes(self.read_aligned::<4>()?))
    }
}

impl XcdrDeserialize for DelimitedCdrDeserializer<'_> {
    fn enter_struct(&mut self) -> crate::error::XcdrResult<()> {
        if let Some(end) = self.end_pos {
            if self.pos < end {
                self.pos = end;
            }
        }
        self.end_pos = None;
        if self.pos >= self.buf.len() {
            return Err(XcdrError::BufferOverflow);
        }
        let header = self.read_u32_ne()?;
        self.header_little_endian = (header & ENDIAN_FLAG) != 0;
        let len = (header & LEN_MASK) as usize;
        let end = self.pos + len;
        if end > self.buf.len() {
            return Err(XcdrError::BufferOverflow);
        }
        self.end_pos = Some(end);
        Ok(())
    }

    fn exit_struct(&mut self) -> crate::error::XcdrResult<()> {
        if let Some(end) = self.end_pos {
            self.pos = end;
        }
        self.end_pos = None;
        Ok(())
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
