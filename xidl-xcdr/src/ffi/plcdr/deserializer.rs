use crate::XcdrDeserialize;

use crate::{
    ffi::{macros::impl_ffi_deserialize_for, XcdrFfiError},
    plcdr::PlcdrDeserializer,
};

#[repr(C)]
pub struct FfiPlcdrDeserializer {
    buf_ptr: *const u8,
    buf_len: usize,
    pos: usize,
}

impl FfiPlcdrDeserializer {
    fn deserializer_buf(&self) -> Result<&[u8], XcdrFfiError> {
        if self.buf_ptr.is_null() && self.buf_len != 0 {
            return Err(XcdrFfiError::NullPointer);
        }
        Ok(unsafe { std::slice::from_raw_parts(self.buf_ptr, self.buf_len) })
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plcdr_deserializer_new(buf_ptr: *const u8, buf_len: usize) -> Self {
        FfiPlcdrDeserializer {
            buf_ptr,
            buf_len,
            pos: 0,
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plcdr_deserializer_position(&self) -> usize {
        self.pos
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plcdr_deserializer_reset(&mut self) {
        self.pos = 0;
    }
}

impl_ffi_deserialize_for!(plcdr_deserializer, FfiPlcdrDeserializer, with_deserializer);

fn with_deserializer<R>(
    self_: &mut FfiPlcdrDeserializer,
    f: impl FnOnce(&mut PlcdrDeserializer<'_>) -> crate::error::XcdrResult<R>,
) -> Result<R, XcdrFfiError> {
    let pos = self_.pos;
    let buf = FfiPlcdrDeserializer::deserializer_buf(self_)?;
    let mut de = PlcdrDeserializer::new(buf);
    de.set_position(pos);
    let out = f(&mut de)?;
    self_.pos = de.position();
    Ok(out)
}
