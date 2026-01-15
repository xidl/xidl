use crate::XcdrSerialize;
use crate::{
    ffi::{macros::impl_ffi_serialize_for, XcdrFfiError},
    plcdr2::Plcdr2Serialize,
};

pub type FfiPlcdr2Serializer = Plcdr2Serialize;

impl FfiPlcdr2Serializer {
    #[unsafe(no_mangle)]
    pub extern "C" fn plcdr2_serializer_new(buf_ptr: *mut u8, buf_len: usize) -> Self {
        Plcdr2Serialize::new(buf_ptr, buf_len)
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plcdr2_serializer_position(&self) -> usize {
        self.pos
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plcdr2_serializer_reset(&mut self) {
        self.pos = 0;
        self.emheader_pos = 0;
        self.field_start = 0;
        self.field_open = false;
        self.pending_member_id = 0;
        self.pending_must_understand = false;
        self.pending_length_code = 0;
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plcdr2_serializer_write_dheader(&mut self, header: u32) -> XcdrFfiError {
        let out = self.write_dheader(header);
        out.into()
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plcdr2_serializer_write_emheader(&mut self, header: u32) -> XcdrFfiError {
        let out = self.write_emheader(header);
        out.into()
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plcdr2_serializer_begin_field(
        &mut self,
        member_id: u32,
        must_understand: bool,
        length_code: u8,
    ) -> XcdrFfiError {
        let out = XcdrSerialize::begin_field(
            self,
            crate::FieldId(member_id),
            must_understand,
            length_code,
        );
        out.into()
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plcdr2_serializer_end_field(&mut self) -> XcdrFfiError {
        let out = XcdrSerialize::end_field(self);
        out.into()
    }
}

impl_ffi_serialize_for!(plcdr2_serializer, FfiPlcdr2Serializer, with_serializer);

fn with_serializer<R>(
    self_: &mut FfiPlcdr2Serializer,
    f: impl FnOnce(&mut Plcdr2Serialize) -> crate::error::XcdrResult<R>,
) -> Result<R, XcdrFfiError> {
    let out = f(self_)?;
    Ok(out)
}
