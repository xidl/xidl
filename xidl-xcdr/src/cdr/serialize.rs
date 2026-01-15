use bytes::BufMut;

use crate::{
    error::{XcdrError, XcdrResult},
    utils::ToNeBytes,
    XcdrSerialize,
};

pub struct CdrSerialize<'a> {
    pub buf: &'a mut [u8],
    pub pos: usize,
    pub do_io: bool,
}

impl<'a> CdrSerialize<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self {
            buf,
            pos: 0,
            do_io: false,
        }
    }
    fn write<T, const N: usize>(&mut self, val: T) -> XcdrResult<()>
    where
        T: ToNeBytes<N>,
    {
        let len = size_of::<T>();
        if self.pos + len > self.buf.len() {
            return Err(XcdrError::BufferOverflow);
        }

        self.align::<T>()?;
        if self.do_io {
            self.buf.put_slice(&val.to_ne_bytes());
        }

        self.pos += len;
        Ok(())
    }

    fn align<T>(&mut self) -> XcdrResult<()> {
        let len = size_of::<T>();
        let align = match len % 4 {
            0 => 0usize,
            v => 4 - v,
        };
        self.pos += align;
        Ok(())
    }
}

impl XcdrSerialize for CdrSerialize<'_> {
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
        if self.pos + buf.len() > self.buf.len() {
            return Err(XcdrError::BufferOverflow);
        }

        if self.do_io {
            self.buf.put_slice(buf);
        }

        self.pos += buf.len();
        Ok(())
    }
}
