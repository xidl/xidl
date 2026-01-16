use crate::{
    error::{XcdrError, XcdrResult},
    utils::{
        align::{align_pos, write_aligned, AlignCdr2},
        ToNeBytes,
    },
    XcdrSerialize,
};

const ENDIAN_FLAG: u32 = 1 << 31;
const LEN_MASK: u32 = !ENDIAN_FLAG;

#[repr(C)]
pub struct DelimitedCdrSerialize {
    pub buf: *mut u8,
    pub len: usize,
    pub pos: usize,
    pub do_io: bool,
    pub(crate) header_pos: usize,
    pub(crate) content_start: usize,
    pub(crate) struct_open: bool,
}

impl DelimitedCdrSerialize {
    pub fn new(buf: *mut u8, len: usize) -> Self {
        Self {
            buf,
            len,
            pos: 0,
            do_io: buf.is_null() || len == 0,
            header_pos: 0,
            content_start: 0,
            struct_open: false,
        }
    }

    fn write_u32_raw(&mut self, val: u32) -> XcdrResult<usize> {
        let len = size_of::<u32>();
        align_pos::<AlignCdr2>(&mut self.pos, len, self.len, true)?;
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
        write_aligned::<AlignCdr2, T, N>(self.buf, self.len, &mut self.pos, self.do_io, val, true)
    }
}

impl XcdrSerialize for DelimitedCdrSerialize {
    fn begin_struct(&mut self) -> XcdrResult<()> {
        if self.struct_open {
            return Err(XcdrError::Message("Struct already open".into()));
        }
        let header_pos = self.write_u32_raw(0)?;
        self.header_pos = header_pos;
        self.content_start = self.pos;
        self.struct_open = true;
        Ok(())
    }

    fn end_struct(&mut self) -> XcdrResult<()> {
        if !self.struct_open {
            return Err(XcdrError::Message("No open struct".into()));
        }
        let header_pos = self.header_pos;
        let start = self.content_start;
        let len = self.pos.saturating_sub(start);
        if len > LEN_MASK as usize {
            return Err(XcdrError::Message("Delimited payload too large".into()));
        }
        let mut header = (len as u32) & LEN_MASK;
        if cfg!(target_endian = "little") {
            header |= ENDIAN_FLAG;
        }
        if self.do_io {
            let src = header.to_ne_bytes();
            unsafe {
                core::ptr::copy(
                    core::ptr::addr_of!(src) as *const u8,
                    self.buf.add(header_pos),
                    src.len(),
                );
            }
        }
        self.struct_open = false;
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
