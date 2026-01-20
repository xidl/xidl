use crate::XcdrDeserializer;

use crate::{
    ffi::{macros::impl_ffi_deserialize_for, XcdrFfiError},
    plcdr2::Plcdr2Deserializer,
};

#[repr(C)]
pub struct FfiPlcdr2Deserializer {
    buf_ptr: *const u8,
    buf_len: usize,
    pos: usize,
}

impl FfiPlcdr2Deserializer {
    fn deserializer_buf(&self) -> Result<&[u8], XcdrFfiError> {
        if self.buf_ptr.is_null() && self.buf_len != 0 {
            return Err(XcdrFfiError::NullPointer);
        }
        Ok(unsafe { std::slice::from_raw_parts(self.buf_ptr, self.buf_len) })
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plcdr2_deserializer_new(buf_ptr: *const u8, buf_len: usize) -> Self {
        FfiPlcdr2Deserializer {
            buf_ptr,
            buf_len,
            pos: 0,
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plcdr2_deserializer_position(&self) -> usize {
        self.pos
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plcdr2_deserializer_reset(&mut self) {
        self.pos = 0;
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plcdr2_deserializer_read_dheader(&mut self, out: *mut u32) -> XcdrFfiError {
        if out.is_null() {
            return XcdrFfiError::NullPointer;
        }
        match with_deserializer(self, |de| de.read_dheader()) {
            Ok(val) => {
                unsafe { *out = val };
                XcdrFfiError::Ok
            }
            Err(err) => err,
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plcdr2_deserializer_read_emheader(&mut self, out: *mut u32) -> XcdrFfiError {
        if out.is_null() {
            return XcdrFfiError::NullPointer;
        }
        match with_deserializer(self, |de| de.read_emheader()) {
            Ok(val) => {
                unsafe { *out = val };
                XcdrFfiError::Ok
            }
            Err(err) => err,
        }
    }
}

impl_ffi_deserialize_for!(
    plcdr2_deserializer,
    FfiPlcdr2Deserializer,
    with_deserializer
);

fn with_deserializer<R>(
    self_: &mut FfiPlcdr2Deserializer,
    f: impl FnOnce(&mut Plcdr2Deserializer<'_>) -> crate::error::XcdrResult<R>,
) -> Result<R, XcdrFfiError> {
    let pos = self_.pos;
    let buf = FfiPlcdr2Deserializer::deserializer_buf(self_)?;
    let mut de = Plcdr2Deserializer::new(buf);
    de.set_position(pos);
    let out = f(&mut de)?;
    self_.pos = de.position();
    Ok(out)
}
