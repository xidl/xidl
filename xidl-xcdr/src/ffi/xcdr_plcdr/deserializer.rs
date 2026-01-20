use crate::XcdrDeserializer;

use crate::{
    ffi::{XcdrFfiError, macros::impl_ffi_deserialize_for},
    xcdr_plcdr::XcdrPlcdrDeserializer,
};

#[repr(C)]
pub struct FfiXcdrPlcdrDeserializer {
    buf_ptr: *const u8,
    buf_len: usize,
    pos: usize,
    field_end: usize,
    field_end_valid: bool,
    expecting_len: bool,
}

impl FfiXcdrPlcdrDeserializer {
    #[unsafe(no_mangle)]
    pub extern "C" fn xcdr_plcdr_deserializer_new(buf_ptr: *const u8, buf_len: usize) -> Self {
        FfiXcdrPlcdrDeserializer {
            buf_ptr,
            buf_len,
            pos: 0,
            field_end: 0,
            field_end_valid: false,
            expecting_len: false,
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn xcdr_plcdr_deserializer_position(&self) -> usize {
        self.pos
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn xcdr_plcdr_deserializer_reset(&mut self) {
        self.pos = 0;
        self.field_end = 0;
        self.field_end_valid = false;
        self.expecting_len = false;
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    #[unsafe(no_mangle)]
    pub extern "C" fn xcdr_plcdr_deserializer_next_field(
        &mut self,
        out_pid: *mut u16,
        out_has_field: *mut bool,
    ) -> XcdrFfiError {
        if out_pid.is_null() || out_has_field.is_null() {
            return XcdrFfiError::NullPointer;
        }
        match with_deserializer(self, |de| de.next_field()) {
            Ok(Some(id)) => {
                unsafe {
                    *out_pid = id.0 as u16;
                    *out_has_field = true;
                }
                XcdrFfiError::Ok
            }
            Ok(None) => {
                unsafe {
                    *out_has_field = false;
                }
                XcdrFfiError::Ok
            }
            Err(err) => err,
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn xcdr_plcdr_deserializer_skip_field(&mut self) -> XcdrFfiError {
        match with_deserializer(self, |de| de.skip_field()) {
            Ok(()) => XcdrFfiError::Ok,
            Err(err) => err,
        }
    }
}

impl_ffi_deserialize_for!(
    xcdr_plcdr_deserializer,
    FfiXcdrPlcdrDeserializer,
    with_deserializer
);

fn with_deserializer<R>(
    self_: &mut FfiXcdrPlcdrDeserializer,
    f: impl FnOnce(&mut XcdrPlcdrDeserializer<'_>) -> crate::error::XcdrResult<R>,
) -> Result<R, XcdrFfiError> {
    let pos = self_.pos;
    let field_end = if self_.field_end_valid {
        Some(self_.field_end)
    } else {
        None
    };
    let expecting_len = self_.expecting_len;
    let buf_ptr = self_.buf_ptr;
    let buf_len = self_.buf_len;
    let buf = if buf_ptr.is_null() && buf_len != 0 {
        return Err(XcdrFfiError::NullPointer);
    } else {
        unsafe { std::slice::from_raw_parts(buf_ptr, buf_len) }
    };
    let mut de = XcdrPlcdrDeserializer::new(buf);
    de.set_position(pos);
    de.set_field_end(field_end);
    de.set_expecting_len(expecting_len);
    let out = f(&mut de)?;
    self_.pos = de.position();
    if let Some(end) = de.field_end() {
        self_.field_end = end;
        self_.field_end_valid = true;
    } else {
        self_.field_end = 0;
        self_.field_end_valid = false;
    }
    self_.expecting_len = de.expecting_len();
    Ok(out)
}
