use bytes::Buf;

use crate::{DeserializeVisitor, SerializeVisitor, XcdrResult};

pub struct CdrSerializer<'a> {
    buf: &'a mut [u8],
    pos: usize,
    no_write: bool,
}

pub struct CdrDeserializer<'a> {
    buf: &'a [u8],
}

impl CdrSerializer<'_> {
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn new(buf: *mut u8, len: usize) -> Self {
        let no_write = buf.is_null() || len == 0;
        let buf = unsafe { std::slice::from_raw_parts_mut(buf, len) };

        Self {
            buf,
            pos: 0,
            no_write,
        }
    }

    fn write<T>(&mut self, value: T) -> XcdrResult<()> {
        let len = size_of::<T>();
        let ptr = &value as *const T as *const u8;
        let value = unsafe { std::slice::from_raw_parts(ptr, len) };
        self.align::<T>();
        let buf = &mut self.buf[self.pos..self.pos + len];
        if !self.no_write {
            buf.copy_from_slice(value);
        }

        self.pos += len;
        Ok(())
    }

    fn align<T>(&mut self) {
        let size = size_of::<T>();
        let advance = match self.pos % size {
            0 => 0,
            1 => 3,
            2 => 2,
            3 => 3,
            _ => 3,
        };
        self.pos += advance;
    }
}

impl CdrDeserializer<'_> {}

macro_rules! declare_ser {
    ($($ty:ty)*) => {
        paste::paste!{
            $(
                fn [<serialize_ $ty>](&mut self, val: $ty) -> XcdrResult<()> {
                    self.write(val)?;
                    Ok(())
                }
            )*
        }
    }
}
impl SerializeVisitor for CdrSerializer<'_> {
    declare_ser! {
        u8 i8 u16 i16 u32 i32 u64 i64 bool f32 f64
    }

    fn serialize_parameter_id(&mut self, _id: u32) -> XcdrResult<()> {
        Ok(())
    }
}

macro_rules! declare_deser {
    ($($id:ty)*) => {
        paste::paste!{
            $(
                fn [<deserialize_ $id _le>](&mut self) -> XcdrResult<$id> {
                    Ok(self.buf.[<get_ $id _le>]())
                }
                fn [<deserialize_ $id _be>](&mut self) -> XcdrResult<$id> {
                    Ok(self.buf.[<get_ $id>]())
                }
            )*
        }
    }
}

impl DeserializeVisitor for CdrDeserializer<'_> {
    fn deserialize_u8(&mut self) -> XcdrResult<u8> {
        Ok(self.buf.get_u8())
    }

    fn deserialize_i8(&mut self) -> XcdrResult<i8> {
        Ok(self.buf.get_i8())
    }

    fn deserialize_bool(&mut self) -> XcdrResult<bool> {
        Ok(self.buf.get_u8() != 0)
    }

    declare_deser! {
        u16 i16 u32 i32 u64 i64 f32 f64
    }

    fn serialize_parameter_id(&mut self) -> XcdrResult<u32> {
        self.deserialize_u32_be()
    }
}
