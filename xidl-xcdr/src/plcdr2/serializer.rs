use crate::{
    error::{XcdrError, XcdrResult},
    utils::{
        align::{write_aligned, AlignCdr2},
        ToNeBytes,
    },
    XcdrSerializer,
};

#[repr(C)]
pub struct Plcdr2Serialize {
    pub buf: *mut u8,
    pub len: usize,
    pub pos: usize,
    pub do_io: bool,
    pub(crate) emheader_pos: usize,
    pub(crate) field_start: usize,
    pub(crate) field_open: bool,
    pub(crate) pending_member_id: u32,
    pub(crate) pending_must_understand: bool,
    pub(crate) pending_length_code: u8,
}

impl Plcdr2Serialize {
    pub fn new(buf: *mut u8, len: usize) -> Self {
        Self {
            buf,
            len,
            pos: 0,
            do_io: buf.is_null() || len == 0,
            emheader_pos: 0,
            field_start: 0,
            field_open: false,
            pending_member_id: 0,
            pending_must_understand: false,
            pending_length_code: 0,
        }
    }

    pub fn write_dheader(&mut self, header: u32) -> XcdrResult<()> {
        self.write(header)
    }

    pub fn write_emheader(&mut self, header: u32) -> XcdrResult<()> {
        self.write(header)
    }

    pub fn begin_field(
        &mut self,
        member_id: u32,
        must_understand: bool,
        length_code: u8,
    ) -> XcdrResult<()> {
        if member_id > 0x0FFF_FFFF {
            return Err(XcdrError::Message("Member id overflow".into()));
        }
        if self.field_open {
            return Err(XcdrError::Message("Field already open".into()));
        }
        let emheader_pos = self.pos;
        self.write_emheader(0)?;
        self.emheader_pos = emheader_pos;
        self.field_start = self.pos;
        self.field_open = true;
        self.pending_member_id = member_id;
        self.pending_must_understand = must_understand;
        self.pending_length_code = length_code;
        Ok(())
    }

    fn length_code_for(&self, len: usize) -> XcdrResult<u8> {
        if self.pending_length_code <= 0x0F {
            return Ok(self.pending_length_code);
        }
        if len <= u8::MAX as usize {
            Ok(1)
        } else if len <= u16::MAX as usize {
            Ok(2)
        } else if len <= u32::MAX as usize {
            Ok(3)
        } else {
            Err(XcdrError::Message("Member too large".into()))
        }
    }

    fn write<T, const N: usize>(&mut self, val: T) -> XcdrResult<()>
    where
        T: ToNeBytes<N>,
    {
        write_aligned::<AlignCdr2, T, N>(self.buf, self.len, &mut self.pos, self.do_io, val, true)
    }
}

impl XcdrSerializer for Plcdr2Serialize {
    fn begin_field(
        &mut self,
        id: crate::FieldId,
        must_understand: bool,
        length_code: u8,
    ) -> XcdrResult<()> {
        self.begin_field(id.0, must_understand, length_code)
    }

    fn end_field(&mut self) -> XcdrResult<()> {
        if !self.field_open {
            return Err(XcdrError::Message("No open field".into()));
        }
        let len = self.pos.saturating_sub(self.field_start);
        let lc = self.length_code_for(len)?;
        if lc > 0x0F {
            return Err(XcdrError::Message("Length code overflow".into()));
        }
        let m_flag = (self.pending_must_understand as u32) << 31;
        let lc_bits = (lc as u32) << 28;
        let emheader1 = m_flag | lc_bits | self.pending_member_id;
        if self.do_io {
            let src = emheader1.to_ne_bytes();
            unsafe {
                core::ptr::copy(
                    core::ptr::addr_of!(src) as *const u8,
                    self.buf.add(self.emheader_pos),
                    src.len(),
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
