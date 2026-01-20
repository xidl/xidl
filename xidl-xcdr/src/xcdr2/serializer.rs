use crate::{
    SerializeKind, XcdrSerializer,
    error::{XcdrError, XcdrResult},
    utils::{
        ToNeBytes,
        align::{AlignCdr2, align_pos, write_aligned},
    },
};

const ENDIAN_FLAG: u32 = 1 << 31;
const LEN_MASK: u32 = !ENDIAN_FLAG;

#[derive(Debug, Clone, Copy)]
enum StructKind {
    Plain,
    Delimited {
        header_pos: usize,
        content_start: usize,
    },
    Plcdr2 {
        emheader_pos: usize,
        field_start: usize,
        field_open: bool,
        pending_member_id: u32,
        pending_must_understand: bool,
        pending_length_code: u8,
    },
}

#[repr(C)]
pub struct Xcdr2Serialize {
    pub buf: *mut u8,
    pub len: usize,
    pub pos: usize,
    pub do_io: bool,
    stack: Vec<StructKind>,
}

impl Xcdr2Serialize {
    pub fn new(buf: *mut u8, len: usize) -> Self {
        Self {
            buf,
            len,
            pos: 0,
            do_io: !(buf.is_null() || len == 0),
            stack: Vec::new(),
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
            let src = val.to_le_bytes();
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

    fn begin_field_plcdr2(
        &mut self,
        member_id: u32,
        must_understand: bool,
        length_code: u8,
    ) -> XcdrResult<()> {
        if member_id > 0x0FFF_FFFF {
            return Err(XcdrError::Message("Member id overflow".into()));
        }
        let idx = match self.stack.len() {
            0 => return Ok(()),
            len => len - 1,
        };
        if matches!(
            self.stack[idx],
            StructKind::Plcdr2 {
                field_open: true,
                ..
            }
        ) {
            return Err(XcdrError::Message("Field already open".into()));
        }
        let header_pos = self.write_u32_raw(0)?;
        let pos = self.pos;
        if let StructKind::Plcdr2 {
            emheader_pos,
            field_start,
            field_open,
            pending_member_id,
            pending_must_understand,
            pending_length_code,
        } = &mut self.stack[idx]
        {
            *emheader_pos = header_pos;
            *field_start = pos;
            *field_open = true;
            *pending_member_id = member_id;
            *pending_must_understand = must_understand;
            *pending_length_code = length_code;
        }
        Ok(())
    }

    fn end_field_plcdr2(&mut self) -> XcdrResult<()> {
        let idx = match self.stack.len() {
            0 => return Ok(()),
            len => len - 1,
        };
        let (
            emheader_pos,
            field_start,
            pending_member_id,
            pending_must_understand,
            pending_length_code,
        ) = match self.stack[idx] {
            StructKind::Plcdr2 {
                emheader_pos,
                field_start,
                field_open,
                pending_member_id,
                pending_must_understand,
                pending_length_code,
            } => {
                if !field_open {
                    return Err(XcdrError::Message("No open field".into()));
                }
                (
                    emheader_pos,
                    field_start,
                    pending_member_id,
                    pending_must_understand,
                    pending_length_code,
                )
            }
            _ => return Ok(()),
        };
        let len = self.pos.saturating_sub(field_start);
        let lc = if pending_length_code <= 0x0F {
            pending_length_code
        } else if len <= u8::MAX as usize {
            1
        } else if len <= u16::MAX as usize {
            2
        } else if len <= u32::MAX as usize {
            3
        } else {
            return Err(XcdrError::Message("Member too large".into()));
        };
        if lc > 0x0F {
            return Err(XcdrError::Message("Length code overflow".into()));
        }
        let m_flag = (pending_must_understand as u32) << 31;
        let lc_bits = (lc as u32) << 28;
        let emheader1 = m_flag | lc_bits | pending_member_id;
        if self.do_io {
            let src = emheader1.to_le_bytes();
            unsafe {
                core::ptr::copy(
                    core::ptr::addr_of!(src) as *const u8,
                    self.buf.add(emheader_pos),
                    src.len(),
                );
            }
        }
        if let StructKind::Plcdr2 { field_open, .. } = &mut self.stack[idx] {
            *field_open = false;
        }
        Ok(())
    }
}

impl XcdrSerializer for Xcdr2Serialize {
    fn begin_struct(&mut self) -> XcdrResult<()> {
        self.begin_struct_with_kind(SerializeKind::PlainCdr2)
    }

    fn begin_struct_with_kind(&mut self, kind: SerializeKind) -> XcdrResult<()> {
        let mode = match kind {
            SerializeKind::DelimitedCdr => {
                let header_pos = self.write_u32_raw(0)?;
                let content_start = self.pos;
                StructKind::Delimited {
                    header_pos,
                    content_start,
                }
            }
            SerializeKind::PlCdr2 => StructKind::Plcdr2 {
                emheader_pos: 0,
                field_start: 0,
                field_open: false,
                pending_member_id: 0,
                pending_must_understand: false,
                pending_length_code: 0,
            },
            _ => StructKind::Plain,
        };
        self.stack.push(mode);
        Ok(())
    }

    fn end_struct(&mut self) -> XcdrResult<()> {
        let Some(state) = self.stack.pop() else {
            return Ok(());
        };
        match state {
            StructKind::Delimited {
                header_pos,
                content_start,
            } => {
                let len = self.pos.saturating_sub(content_start);
                if len > LEN_MASK as usize {
                    return Err(XcdrError::Message("Delimited payload too large".into()));
                }
                let mut header = (len as u32) & LEN_MASK;
                header |= ENDIAN_FLAG;
                if self.do_io {
                    let src = header.to_le_bytes();
                    unsafe {
                        core::ptr::copy(
                            core::ptr::addr_of!(src) as *const u8,
                            self.buf.add(header_pos),
                            src.len(),
                        );
                    }
                }
            }
            StructKind::Plcdr2 { field_open, .. } => {
                if field_open {
                    return Err(XcdrError::Message("Field still open".into()));
                }
            }
            StructKind::Plain => {}
        }
        Ok(())
    }

    fn begin_field(
        &mut self,
        id: crate::FieldId,
        must_understand: bool,
        length_code: u8,
    ) -> XcdrResult<()> {
        self.begin_field_plcdr2(id.0, must_understand, length_code)
    }

    fn end_field(&mut self) -> XcdrResult<()> {
        self.end_field_plcdr2()
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
