use crate::XcdrDeserializer;

use crate::{
    delimited_cdr::DelimitedCdrDeserializer,
    ffi::{macros::impl_ffi_deserialize_for, XcdrFfiError},
};

#[repr(C)]
pub struct FfiDelimitedCdrDeserializer {
    buf_ptr: *const u8,
    buf_len: usize,
    pos: usize,
    end_pos: usize,
    end_pos_valid: bool,
    header_little_endian: bool,
}

impl FfiDelimitedCdrDeserializer {
    #[unsafe(no_mangle)]
    pub extern "C" fn delimited_cdr_deserializer_new(buf_ptr: *const u8, buf_len: usize) -> Self {
        FfiDelimitedCdrDeserializer {
            buf_ptr,
            buf_len,
            pos: 0,
            end_pos: 0,
            end_pos_valid: false,
            header_little_endian: cfg!(target_endian = "little"),
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn delimited_cdr_deserializer_position(&self) -> usize {
        self.pos
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn delimited_cdr_deserializer_reset(&mut self) {
        self.pos = 0;
        self.end_pos = 0;
        self.end_pos_valid = false;
        self.header_little_endian = cfg!(target_endian = "little");
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn delimited_cdr_deserializer_enter_struct(&mut self) -> XcdrFfiError {
        match with_deserializer(self, |de| de.enter_struct()) {
            Ok(()) => XcdrFfiError::Ok,
            Err(err) => err,
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn delimited_cdr_deserializer_exit_struct(&mut self) -> XcdrFfiError {
        match with_deserializer(self, |de| de.exit_struct()) {
            Ok(()) => XcdrFfiError::Ok,
            Err(err) => err,
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn delimited_cdr_deserializer_header_little_endian(&self) -> bool {
        self.header_little_endian
    }
}

impl_ffi_deserialize_for!(
    delimited_cdr_deserializer,
    FfiDelimitedCdrDeserializer,
    with_deserializer
);

fn with_deserializer<R>(
    self_: &mut FfiDelimitedCdrDeserializer,
    f: impl FnOnce(&mut DelimitedCdrDeserializer<'_>) -> crate::error::XcdrResult<R>,
) -> Result<R, XcdrFfiError> {
    let pos = self_.pos;
    let end_pos = if self_.end_pos_valid {
        Some(self_.end_pos)
    } else {
        None
    };
    let header_little_endian = self_.header_little_endian;
    let buf_ptr = self_.buf_ptr;
    let buf_len = self_.buf_len;
    let buf = if buf_ptr.is_null() && buf_len != 0 {
        return Err(XcdrFfiError::NullPointer);
    } else {
        unsafe { std::slice::from_raw_parts(buf_ptr, buf_len) }
    };
    let mut de = DelimitedCdrDeserializer::new(buf);
    de.set_position(pos);
    de.set_end_position(end_pos);
    de.header_little_endian = header_little_endian;
    let out = f(&mut de)?;
    self_.pos = de.position();
    if let Some(end) = de.end_position() {
        self_.end_pos = end;
        self_.end_pos_valid = true;
    } else {
        self_.end_pos = 0;
        self_.end_pos_valid = false;
    }
    self_.header_little_endian = de.header_little_endian;
    Ok(out)
}
