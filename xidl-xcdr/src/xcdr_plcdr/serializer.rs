use crate::{
    error::{XcdrError, XcdrResult},
    utils::ToNeBytes,
    FieldId, XcdrSerializer,
};

#[repr(C)]
pub struct XcdrPlcdrSerialize {
    pub buf: *mut u8,
    pub len: usize,
    pub pos: usize,
    pub do_io: bool,
    pub(crate) field_len_pos: usize,
    pub(crate) field_start_pos: usize,
    pub(crate) field_open: bool,
}

impl XcdrPlcdrSerialize {
    pub fn new(buf: *mut u8, len: usize) -> Self {
        Self {
            buf,
            len,
            pos: 0,
            do_io: !(buf.is_null() || len == 0),
            field_len_pos: 0,
            field_start_pos: 0,
            field_open: false,
        }
    }

    fn align_len(&mut self, len: usize) -> XcdrResult<()> {
        let align = match len % 4 {
            0 => 0usize,
            v => 4 - v,
        };
        if self.pos + align > self.len {
            return Err(XcdrError::BufferOverflow);
        }
        self.pos += align;
        Ok(())
    }

    fn write_u16_raw(&mut self, val: u16) -> XcdrResult<usize> {
        let len = size_of::<u16>();
        self.align_len(len)?;
        if self.pos + len > self.len {
            return Err(XcdrError::BufferOverflow);
        }
        let write_pos = self.pos;
        if self.do_io {
            let src = val.to_ne_bytes();
            unsafe {
                core::ptr::copy(
                    core::ptr::addr_of!(src) as *const u8,
                    self.buf.add(write_pos),
                    src.len(),
                );
            }
        }
        self.pos += len;
        Ok(write_pos)
    }

    fn write<T, const N: usize>(&mut self, val: T) -> XcdrResult<()>
    where
        T: ToNeBytes<N>,
    {
        let len = size_of::<T>();
        if self.pos + len > self.len {
            return Err(XcdrError::BufferOverflow);
        }

        self.align_len(len)?;
        if self.do_io {
            let src = &val.to_ne_bytes();

            unsafe {
                core::ptr::copy(
                    core::ptr::addr_of!(*src) as *const u8,
                    self.buf.add(self.pos),
                    src.len(),
                );
            }
        }

        self.pos += len;
        Ok(())
    }
}

impl XcdrSerializer for XcdrPlcdrSerialize {
    fn begin_field(
        &mut self,
        id: FieldId,
        _must_understand: bool,
        _length_code: u8,
    ) -> XcdrResult<()> {
        if self.field_open {
            return Err(XcdrError::Message("Field already open".into()));
        }
        if id.0 > u16::MAX as u32 {
            return Err(XcdrError::Message("Field id overflow".into()));
        }
        self.write_u16_raw(id.0 as u16)?;
        let len_pos = self.write_u16_raw(0)?;
        self.field_len_pos = len_pos;
        self.field_start_pos = self.pos;
        self.field_open = true;
        Ok(())
    }

    fn end_field(&mut self) -> XcdrResult<()> {
        if !self.field_open {
            return Err(XcdrError::Message("No open field".into()));
        }
        let len_pos = self.field_len_pos;
        let start = self.field_start_pos;
        let len = self.pos.saturating_sub(start);
        if len > u16::MAX as usize {
            return Err(XcdrError::Message("Field too large".into()));
        }
        if self.do_io {
            let len_bytes = (len as u16).to_ne_bytes();
            unsafe {
                core::ptr::copy(
                    core::ptr::addr_of!(len_bytes) as *const u8,
                    self.buf.add(len_pos),
                    len_bytes.len(),
                );
            }
        }
        self.field_open = false;
        Ok(())
    }

    fn write_bool(&mut self, val: bool) -> XcdrResult<()> {
        self.write(val)
    }
    fn write_u8(&mut self, val: u8) -> XcdrResult<()> {
        self.write(val)
    }
    fn write_i8(&mut self, val: i8) -> XcdrResult<()> {
        self.write(val)
    }
    fn write_u16(&mut self, val: u16) -> XcdrResult<()> {
        self.write(val)
    }
    fn write_i16(&mut self, val: i16) -> XcdrResult<()> {
        self.write(val)
    }
    fn write_u32(&mut self, val: u32) -> XcdrResult<()> {
        self.write(val)
    }
    fn write_i32(&mut self, val: i32) -> XcdrResult<()> {
        self.write(val)
    }
    fn write_u64(&mut self, val: u64) -> XcdrResult<()> {
        self.write(val)
    }
    fn write_i64(&mut self, val: i64) -> XcdrResult<()> {
        self.write(val)
    }
    fn write_f32(&mut self, val: f32) -> XcdrResult<()> {
        self.write(val)
    }
    fn write_f64(&mut self, val: f64) -> XcdrResult<()> {
        self.write(val)
    }
    fn write_bytes(&mut self, buf: &[u8]) -> XcdrResult<()> {
        if self.pos + buf.len() > self.len {
            return Err(XcdrError::BufferOverflow);
        }

        if self.do_io {
            unsafe {
                std::ptr::copy(buf.as_ptr(), self.buf.add(self.pos), buf.len());
            }
        }

        self.pos += buf.len();
        Ok(())
    }
}
