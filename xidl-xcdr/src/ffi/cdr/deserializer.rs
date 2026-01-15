use crate::XcdrDeserialize;

use crate::{cdr::CdrDeserializer, ffi::macros::impl_ffi_deserialize_for, ffi::XcdrFfiError};

#[repr(C)]
pub struct FfiCdrDeserializer {
    buf_ptr: *const u8,
    buf_len: usize,
    pos: usize,
}

impl FfiCdrDeserializer {
    fn deserializer_buf(&self) -> Result<&[u8], XcdrFfiError> {
        if self.buf_ptr.is_null() && self.buf_len != 0 {
            return Err(XcdrFfiError::NullPointer);
        }
        Ok(unsafe { std::slice::from_raw_parts(self.buf_ptr, self.buf_len) })
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn cdr_deserializer_new(buf_ptr: *const u8, buf_len: usize) -> Self {
        FfiCdrDeserializer {
            buf_ptr,
            buf_len,
            pos: 0,
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn cdr_deserializer_position(&self) -> usize {
        self.pos
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn cdr_deserializer_reset(&mut self) {
        self.pos = 0;
    }
}

impl_ffi_deserialize_for!(cdr_deserializer, FfiCdrDeserializer, with_deserializer);

fn with_deserializer<R>(
    self_: &mut FfiCdrDeserializer,
    f: impl FnOnce(&mut CdrDeserializer<'_>) -> crate::error::XcdrResult<R>,
) -> Result<R, XcdrFfiError> {
    let pos = self_.pos;
    let buf = FfiCdrDeserializer::deserializer_buf(self_)?;
    let mut de = CdrDeserializer::new(buf);
    de.set_position(pos);
    let out = f(&mut de)?;
    self_.pos = de.position();
    Ok(out)
}
