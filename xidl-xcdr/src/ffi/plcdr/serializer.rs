use crate::XcdrSerializer;
use crate::{
    ffi::{XcdrFfiError, macros::impl_ffi_serialize_for},
    plcdr::PlcdrSerialize,
};

pub type FfiPlcdrSerializer = PlcdrSerialize;

impl FfiPlcdrSerializer {
    #[unsafe(no_mangle)]
    pub extern "C" fn plcdr_serializer_new(buf_ptr: *mut u8, buf_len: usize) -> Self {
        let do_io = !(buf_ptr.is_null() || buf_len == 0);
        Self {
            buf: buf_ptr,
            len: buf_len,
            pos: 0,
            do_io,
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plcdr_serializer_position(&self) -> usize {
        self.pos
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plcdr_serializer_reset(&mut self) {
        self.pos = 0;
    }
}

impl_ffi_serialize_for!(plcdr_serializer, FfiPlcdrSerializer, with_serializer);

fn with_serializer<R>(
    self_: &mut FfiPlcdrSerializer,
    f: impl FnOnce(&mut PlcdrSerialize) -> crate::error::XcdrResult<R>,
) -> Result<R, XcdrFfiError> {
    let out = f(self_)?;
    Ok(out)
}
