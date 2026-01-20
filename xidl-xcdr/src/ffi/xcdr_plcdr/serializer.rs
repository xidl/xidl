use crate::XcdrSerializer;
use crate::{
    ffi::{macros::impl_ffi_serialize_for, XcdrFfiError},
    xcdr_plcdr::XcdrPlcdrSerialize,
    FieldId,
};

pub type FfiXcdrPlcdrSerializer = XcdrPlcdrSerialize;

impl FfiXcdrPlcdrSerializer {
    #[unsafe(no_mangle)]
    pub extern "C" fn xcdr_plcdr_serializer_new(buf_ptr: *mut u8, buf_len: usize) -> Self {
        XcdrPlcdrSerialize::new(buf_ptr, buf_len)
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn xcdr_plcdr_serializer_position(&self) -> usize {
        self.pos
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn xcdr_plcdr_serializer_reset(&mut self) {
        self.pos = 0;
        self.field_len_pos = 0;
        self.field_start_pos = 0;
        self.field_open = false;
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn xcdr_plcdr_serializer_begin_field(&mut self, pid: u16) -> XcdrFfiError {
        let out = XcdrSerializer::begin_field(self, FieldId(pid as u32), false, 0);
        out.into()
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn xcdr_plcdr_serializer_end_field(&mut self) -> XcdrFfiError {
        let out = XcdrSerializer::end_field(self);
        out.into()
    }
}

impl_ffi_serialize_for!(
    xcdr_plcdr_serializer,
    FfiXcdrPlcdrSerializer,
    with_serializer
);

fn with_serializer<R>(
    self_: &mut FfiXcdrPlcdrSerializer,
    f: impl FnOnce(&mut XcdrPlcdrSerialize) -> crate::error::XcdrResult<R>,
) -> Result<R, XcdrFfiError> {
    let out = f(self_)?;
    Ok(out)
}
