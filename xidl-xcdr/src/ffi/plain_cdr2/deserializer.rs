use crate::XcdrDeserializer;

use crate::{
    ffi::{XcdrFfiError, macros::impl_ffi_deserialize_for},
    plain_cdr2::PlainCdr2Deserializer,
};

#[repr(C)]
pub struct FfiPlainCdr2Deserializer {
    buf_ptr: *const u8,
    buf_len: usize,
    pos: usize,
}

impl FfiPlainCdr2Deserializer {
    fn deserializer_buf(&self) -> Result<&[u8], XcdrFfiError> {
        if self.buf_ptr.is_null() && self.buf_len != 0 {
            return Err(XcdrFfiError::NullPointer);
        }
        Ok(unsafe { std::slice::from_raw_parts(self.buf_ptr, self.buf_len) })
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plain_cdr2_deserializer_new(buf_ptr: *const u8, buf_len: usize) -> Self {
        FfiPlainCdr2Deserializer {
            buf_ptr,
            buf_len,
            pos: 0,
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plain_cdr2_deserializer_position(&self) -> usize {
        self.pos
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plain_cdr2_deserializer_reset(&mut self) {
        self.pos = 0;
    }
}

impl_ffi_deserialize_for!(
    plain_cdr2_deserializer,
    FfiPlainCdr2Deserializer,
    with_deserializer
);

fn with_deserializer<R>(
    self_: &mut FfiPlainCdr2Deserializer,
    f: impl FnOnce(&mut PlainCdr2Deserializer<'_>) -> crate::error::XcdrResult<R>,
) -> Result<R, XcdrFfiError> {
    let pos = self_.pos;
    let buf = FfiPlainCdr2Deserializer::deserializer_buf(self_)?;
    let mut de = PlainCdr2Deserializer::new(buf);
    de.set_position(pos);
    let out = f(&mut de)?;
    self_.pos = de.position();
    Ok(out)
}
