use crate::XcdrSerializer;
use crate::{
    delimited_cdr::DelimitedCdrSerialize,
    ffi::{XcdrFfiError, macros::impl_ffi_serialize_for},
};

pub type FfiDelimitedCdrSerializer = DelimitedCdrSerialize;

impl FfiDelimitedCdrSerializer {
    #[unsafe(no_mangle)]
    pub extern "C" fn delimited_cdr_serializer_new(buf_ptr: *mut u8, buf_len: usize) -> Self {
        DelimitedCdrSerialize::new(buf_ptr, buf_len)
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn delimited_cdr_serializer_position(&self) -> usize {
        self.pos
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn delimited_cdr_serializer_reset(&mut self) {
        self.pos = 0;
        self.header_pos = 0;
        self.content_start = 0;
        self.struct_open = false;
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn delimited_cdr_serializer_begin_struct(&mut self) -> XcdrFfiError {
        let out = XcdrSerializer::begin_struct(self);
        out.into()
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn delimited_cdr_serializer_end_struct(&mut self) -> XcdrFfiError {
        let out = XcdrSerializer::end_struct(self);
        out.into()
    }
}

impl_ffi_serialize_for!(
    delimited_cdr_serializer,
    FfiDelimitedCdrSerializer,
    with_serializer
);

fn with_serializer<R>(
    self_: &mut FfiDelimitedCdrSerializer,
    f: impl FnOnce(&mut DelimitedCdrSerialize) -> crate::error::XcdrResult<R>,
) -> Result<R, XcdrFfiError> {
    let out = f(self_)?;
    Ok(out)
}
