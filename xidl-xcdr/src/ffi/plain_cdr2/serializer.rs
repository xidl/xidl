use crate::XcdrSerializer;
use crate::{
    ffi::{macros::impl_ffi_serialize_for, XcdrFfiError},
    plain_cdr2::PlainCdr2Serialize,
};

pub type FfiPlainCdr2Serializer = PlainCdr2Serialize;

impl FfiPlainCdr2Serializer {
    #[unsafe(no_mangle)]
    pub extern "C" fn plain_cdr2_serializer_new(buf_ptr: *mut u8, buf_len: usize) -> Self {
        let do_io = buf_ptr.is_null() || buf_len == 0;
        Self {
            buf: buf_ptr,
            len: buf_len,
            pos: 0,
            do_io,
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plain_cdr2_serializer_position(&self) -> usize {
        self.pos
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plain_cdr2_serializer_reset(&mut self) {
        self.pos = 0;
    }
}

impl_ffi_serialize_for!(
    plain_cdr2_serializer,
    FfiPlainCdr2Serializer,
    with_serializer
);

fn with_serializer<R>(
    self_: &mut FfiPlainCdr2Serializer,
    f: impl FnOnce(&mut PlainCdr2Serialize) -> crate::error::XcdrResult<R>,
) -> Result<R, XcdrFfiError> {
    let out = f(self_)?;
    Ok(out)
}
