use crate::utils::{
    FromBytes,
    align::{Align4, read_aligned},
};
use crate::{FieldId, XcdrDeserializer, error::XcdrError};

pub struct XcdrPlcdrDeserializer<'a> {
    buf: &'a [u8],
    pos: usize,
    field_end: Option<usize>,
    expecting_len: bool,
}

impl<'a> XcdrPlcdrDeserializer<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self {
            buf,
            pos: 0,
            field_end: None,
            expecting_len: false,
        }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub fn field_end(&self) -> Option<usize> {
        self.field_end
    }

    pub fn set_field_end(&mut self, field_end: Option<usize>) {
        self.field_end = field_end;
    }

    pub fn expecting_len(&self) -> bool {
        self.expecting_len
    }

    pub fn set_expecting_len(&mut self, expecting_len: bool) {
        self.expecting_len = expecting_len;
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

    fn read_field_len(&mut self, len: usize) -> crate::error::XcdrResult<()> {
        let end = self.pos + len;
        if end > self.buf.len() {
            return Err(XcdrError::BufferOverflow);
        }
        self.field_end = Some(end);
        self.expecting_len = false;
        Ok(())
    }

    fn ensure_len_read(&self) -> crate::error::XcdrResult<()> {
        if self.expecting_len {
            return Err(XcdrError::Message("Field length not read".into()));
        }
        Ok(())
    }

    fn read_u16_ne(&mut self) -> crate::error::XcdrResult<u16> {
        Ok(u16::from_ne_bytes(self.read_aligned::<2>()?))
    }
}

impl XcdrDeserializer for XcdrPlcdrDeserializer<'_> {
    fn next_field(&mut self) -> crate::error::XcdrResult<Option<FieldId>> {
        if let Some(end) = self.field_end {
            if self.pos < end {
                self.pos = end;
            }
        }
        self.field_end = None;
        self.expecting_len = false;
        if self.pos >= self.buf.len() {
            return Ok(None);
        }
        let pid = self.read_u16_ne()?;
        self.expecting_len = true;
        Ok(Some(FieldId(pid as u32)))
    }

    fn skip_field(&mut self) -> crate::error::XcdrResult<()> {
        if self.expecting_len {
            let len = self.read_u16_ne()? as usize;
            self.read_field_len(len)?;
        }
        if let Some(end) = self.field_end {
            self.pos = end;
        }
        self.field_end = None;
        Ok(())
    }

    fn read_u8(&mut self) -> crate::error::XcdrResult<u8> {
        self.ensure_len_read()?;
        self.read_num_be()
    }

    fn read_i8(&mut self) -> crate::error::XcdrResult<i8> {
        self.ensure_len_read()?;
        self.read_num_be()
    }

    fn read_bool(&mut self) -> crate::error::XcdrResult<bool> {
        self.ensure_len_read()?;
        self.read_num_be()
    }

    fn read_u16_le(&mut self) -> crate::error::XcdrResult<u16> {
        let val = self.read_num_le()?;
        if self.expecting_len {
            self.read_field_len(val as usize)?;
        }
        Ok(val)
    }

    fn read_u16_be(&mut self) -> crate::error::XcdrResult<u16> {
        let val = self.read_num_be()?;
        if self.expecting_len {
            self.read_field_len(val as usize)?;
        }
        Ok(val)
    }

    fn read_i16_le(&mut self) -> crate::error::XcdrResult<i16> {
        self.ensure_len_read()?;
        self.read_num_le()
    }

    fn read_i16_be(&mut self) -> crate::error::XcdrResult<i16> {
        self.ensure_len_read()?;
        self.read_num_be()
    }

    fn read_u32_le(&mut self) -> crate::error::XcdrResult<u32> {
        self.ensure_len_read()?;
        self.read_num_le()
    }

    fn read_u32_be(&mut self) -> crate::error::XcdrResult<u32> {
        self.ensure_len_read()?;
        self.read_num_be()
    }

    fn read_i32_le(&mut self) -> crate::error::XcdrResult<i32> {
        self.ensure_len_read()?;
        self.read_num_le()
    }

    fn read_i32_be(&mut self) -> crate::error::XcdrResult<i32> {
        self.ensure_len_read()?;
        self.read_num_be()
    }

    fn read_u64_le(&mut self) -> crate::error::XcdrResult<u64> {
        self.ensure_len_read()?;
        self.read_num_le()
    }

    fn read_u64_be(&mut self) -> crate::error::XcdrResult<u64> {
        self.ensure_len_read()?;
        self.read_num_be()
    }

    fn read_i64_le(&mut self) -> crate::error::XcdrResult<i64> {
        self.ensure_len_read()?;
        self.read_num_le()
    }

    fn read_i64_be(&mut self) -> crate::error::XcdrResult<i64> {
        self.ensure_len_read()?;
        self.read_num_be()
    }

    fn read_f32_le(&mut self) -> crate::error::XcdrResult<f32> {
        self.ensure_len_read()?;
        self.read_num_le()
    }

    fn read_f32_be(&mut self) -> crate::error::XcdrResult<f32> {
        self.ensure_len_read()?;
        self.read_num_be()
    }

    fn read_f64_le(&mut self) -> crate::error::XcdrResult<f64> {
        self.ensure_len_read()?;
        self.read_num_le()
    }

    fn read_f64_be(&mut self) -> crate::error::XcdrResult<f64> {
        self.ensure_len_read()?;
        self.read_num_be()
    }

    fn read_bytes(&mut self, out: &mut [u8]) -> crate::error::XcdrResult<()> {
        self.ensure_len_read()?;
        self.read_raw(out)
    }
}
