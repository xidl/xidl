macro_rules! impl_ffi_serialize_for {
    ($prefix:ident, $ctx_ty:ty, $with_fn:ident) => {
        paste::paste! {
            impl $ctx_ty {
                #[unsafe(no_mangle)]
                pub extern "C" fn  [<$prefix _write_u8>](&mut self, val: u8) -> XcdrFfiError {
                    match $with_fn(self, |ser| ser.write_u8(val)) {
                        Ok(()) => XcdrFfiError::Ok,
                        Err(err) => err,
                    }
                }

                #[unsafe(no_mangle)]
                pub extern "C" fn  [<$prefix _write_i8>](&mut self, val: i8) -> XcdrFfiError {
                    match $with_fn(self, |ser| ser.write_i8(val)) {
                        Ok(()) => XcdrFfiError::Ok,
                        Err(err) => err,
                    }
                }

                #[unsafe(no_mangle)]
                pub extern "C" fn  [<$prefix _write_bool>](&mut self, val: bool) -> XcdrFfiError {
                    match $with_fn(self, |ser| ser.write_bool(val)) {
                        Ok(()) => XcdrFfiError::Ok,
                        Err(err) => err,
                    }
                }

                #[unsafe(no_mangle)]
                pub extern "C" fn  [<$prefix _write_u16>](&mut self, val: u16) -> XcdrFfiError {
                    match $with_fn(self, |ser| ser.write_u16(val)) {
                        Ok(()) => XcdrFfiError::Ok,
                        Err(err) => err,
                    }
                }

                #[unsafe(no_mangle)]
                pub extern "C" fn  [<$prefix _write_i16>](&mut self, val: i16) -> XcdrFfiError {
                    match $with_fn(self, |ser| ser.write_i16(val)) {
                        Ok(()) => XcdrFfiError::Ok,
                        Err(err) => err,
                    }
                }

                #[unsafe(no_mangle)]
                pub extern "C" fn  [<$prefix _write_u32>](&mut self, val: u32) -> XcdrFfiError {
                    match $with_fn(self, |ser| ser.write_u32(val)) {
                        Ok(()) => XcdrFfiError::Ok,
                        Err(err) => err,
                    }
                }

                #[unsafe(no_mangle)]
                pub extern "C" fn  [<$prefix _write_i32>](&mut self, val: i32) -> XcdrFfiError {
                    match $with_fn(self, |ser| ser.write_i32(val)) {
                        Ok(()) => XcdrFfiError::Ok,
                        Err(err) => err,
                    }
                }

                #[unsafe(no_mangle)]
                pub extern "C" fn  [<$prefix _write_u64>](&mut self, val: u64) -> XcdrFfiError {
                    match $with_fn(self, |ser| ser.write_u64(val)) {
                        Ok(()) => XcdrFfiError::Ok,
                        Err(err) => err,
                    }
                }

                #[unsafe(no_mangle)]
                pub extern "C" fn  [<$prefix _write_i64>](&mut self, val: i64) -> XcdrFfiError {
                    match $with_fn(self, |ser| ser.write_i64(val)) {
                        Ok(()) => XcdrFfiError::Ok,
                        Err(err) => err,
                    }
                }

                #[unsafe(no_mangle)]
                pub extern "C" fn  [<$prefix _write_f32>](&mut self, val: f32) -> XcdrFfiError {
                    match $with_fn(self, |ser| ser.write_f32(val)) {
                        Ok(()) => XcdrFfiError::Ok,
                        Err(err) => err,
                    }
                }

                #[unsafe(no_mangle)]
                pub extern "C" fn  [<$prefix _write_f64>](&mut self, val: f64) -> XcdrFfiError {
                    match $with_fn(self, |ser| ser.write_f64(val)) {
                        Ok(()) => XcdrFfiError::Ok,
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn  [<$prefix _write_bytes>](
                    &mut self,
                    buf_ptr: *const u8,
                    buf_len: usize,
                ) -> XcdrFfiError {
                    if buf_ptr.is_null() && buf_len != 0 {
                        return XcdrFfiError::NullPointer;
                    }
                    let buf = unsafe { std::slice::from_raw_parts(buf_ptr, buf_len) };
                    match $with_fn(self, |ser| ser.write_bytes(buf)) {
                        Ok(()) => XcdrFfiError::Ok,
                        Err(err) => err,
                    }
                }
            }
        }
    };
}

macro_rules! impl_ffi_deserialize_for {
    ($prefix:ident, $ctx_ty:ty, $with_fn:ident) => {
        paste::paste! {
            impl $ctx_ty {
                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_u8>](&mut self, out: *mut u8) -> XcdrFfiError {
                    if out.is_null() {
                        return XcdrFfiError::NullPointer;
                    }
                    match $with_fn(self, |de| de.read_u8()) {
                        Ok(val) => {
                            unsafe { *out = val };
                            XcdrFfiError::Ok
                        }
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_i8>](&mut self, out: *mut i8) -> XcdrFfiError {
                    if out.is_null() {
                        return XcdrFfiError::NullPointer;
                    }
                    match $with_fn(self, |de| de.read_i8()) {
                        Ok(val) => {
                            unsafe { *out = val };
                            XcdrFfiError::Ok
                        }
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_bool>](&mut self, out: *mut bool) -> XcdrFfiError {
                    if out.is_null() {
                        return XcdrFfiError::NullPointer;
                    }
                    match $with_fn(self, |de| de.read_bool()) {
                        Ok(val) => {
                            unsafe { *out = val };
                            XcdrFfiError::Ok
                        }
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_u16_le>](&mut self, out: *mut u16) -> XcdrFfiError {
                    if out.is_null() {
                        return XcdrFfiError::NullPointer;
                    }
                    match $with_fn(self, |de| de.read_u16_le()) {
                        Ok(val) => {
                            unsafe { *out = val };
                            XcdrFfiError::Ok
                        }
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_u16_be>](&mut self, out: *mut u16) -> XcdrFfiError {
                    if out.is_null() {
                        return XcdrFfiError::NullPointer;
                    }
                    match $with_fn(self, |de| de.read_u16_be()) {
                        Ok(val) => {
                            unsafe { *out = val };
                            XcdrFfiError::Ok
                        }
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_i16_le>](&mut self, out: *mut i16) -> XcdrFfiError {
                    if out.is_null() {
                        return XcdrFfiError::NullPointer;
                    }
                    match $with_fn(self, |de| de.read_i16_le()) {
                        Ok(val) => {
                            unsafe { *out = val };
                            XcdrFfiError::Ok
                        }
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_i16_be>](&mut self, out: *mut i16) -> XcdrFfiError {
                    if out.is_null() {
                        return XcdrFfiError::NullPointer;
                    }
                    match $with_fn(self, |de| de.read_i16_be()) {
                        Ok(val) => {
                            unsafe { *out = val };
                            XcdrFfiError::Ok
                        }
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_u32_le>](&mut self, out: *mut u32) -> XcdrFfiError {
                    if out.is_null() {
                        return XcdrFfiError::NullPointer;
                    }
                    match $with_fn(self, |de| de.read_u32_le()) {
                        Ok(val) => {
                            unsafe { *out = val };
                            XcdrFfiError::Ok
                        }
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_u32_be>](&mut self, out: *mut u32) -> XcdrFfiError {
                    if out.is_null() {
                        return XcdrFfiError::NullPointer;
                    }
                    match $with_fn(self, |de| de.read_u32_be()) {
                        Ok(val) => {
                            unsafe { *out = val };
                            XcdrFfiError::Ok
                        }
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_i32_le>](&mut self, out: *mut i32) -> XcdrFfiError {
                    if out.is_null() {
                        return XcdrFfiError::NullPointer;
                    }
                    match $with_fn(self, |de| de.read_i32_le()) {
                        Ok(val) => {
                            unsafe { *out = val };
                            XcdrFfiError::Ok
                        }
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_i32_be>](&mut self, out: *mut i32) -> XcdrFfiError {
                    if out.is_null() {
                        return XcdrFfiError::NullPointer;
                    }
                    match $with_fn(self, |de| de.read_i32_be()) {
                        Ok(val) => {
                            unsafe { *out = val };
                            XcdrFfiError::Ok
                        }
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_u64_le>](&mut self, out: *mut u64) -> XcdrFfiError {
                    if out.is_null() {
                        return XcdrFfiError::NullPointer;
                    }
                    match $with_fn(self, |de| de.read_u64_le()) {
                        Ok(val) => {
                            unsafe { *out = val };
                            XcdrFfiError::Ok
                        }
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_u64_be>](&mut self, out: *mut u64) -> XcdrFfiError {
                    if out.is_null() {
                        return XcdrFfiError::NullPointer;
                    }
                    match $with_fn(self, |de| de.read_u64_be()) {
                        Ok(val) => {
                            unsafe { *out = val };
                            XcdrFfiError::Ok
                        }
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_i64_le>](&mut self, out: *mut i64) -> XcdrFfiError {
                    if out.is_null() {
                        return XcdrFfiError::NullPointer;
                    }
                    match $with_fn(self, |de| de.read_i64_le()) {
                        Ok(val) => {
                            unsafe { *out = val };
                            XcdrFfiError::Ok
                        }
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_i64_be>](&mut self, out: *mut i64) -> XcdrFfiError {
                    if out.is_null() {
                        return XcdrFfiError::NullPointer;
                    }
                    match $with_fn(self, |de| de.read_i64_be()) {
                        Ok(val) => {
                            unsafe { *out = val };
                            XcdrFfiError::Ok
                        }
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_f32_le>](&mut self, out: *mut f32) -> XcdrFfiError {
                    if out.is_null() {
                        return XcdrFfiError::NullPointer;
                    }
                    match $with_fn(self, |de| de.read_f32_le()) {
                        Ok(val) => {
                            unsafe { *out = val };
                            XcdrFfiError::Ok
                        }
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_f32_be>](&mut self, out: *mut f32) -> XcdrFfiError {
                    if out.is_null() {
                        return XcdrFfiError::NullPointer;
                    }
                    match $with_fn(self, |de| de.read_f32_be()) {
                        Ok(val) => {
                            unsafe { *out = val };
                            XcdrFfiError::Ok
                        }
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_f64_le>](&mut self, out: *mut f64) -> XcdrFfiError {
                    if out.is_null() {
                        return XcdrFfiError::NullPointer;
                    }
                    match $with_fn(self, |de| de.read_f64_le()) {
                        Ok(val) => {
                            unsafe { *out = val };
                            XcdrFfiError::Ok
                        }
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_f64_be>](&mut self, out: *mut f64) -> XcdrFfiError {
                    if out.is_null() {
                        return XcdrFfiError::NullPointer;
                    }
                    match $with_fn(self, |de| de.read_f64_be()) {
                        Ok(val) => {
                            unsafe { *out = val };
                            XcdrFfiError::Ok
                        }
                        Err(err) => err,
                    }
                }

                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                #[unsafe(no_mangle)]
                pub extern "C" fn [<$prefix _read_bytes>](
                    &mut self,
                    out_ptr: *mut u8,
                    out_len: usize,
                ) -> XcdrFfiError {
                    if out_ptr.is_null() && out_len != 0 {
                        return XcdrFfiError::NullPointer;
                    }
                    let out = unsafe { std::slice::from_raw_parts_mut(out_ptr, out_len) };
                    match $with_fn(self, |de| de.read_bytes(out)) {
                        Ok(()) => XcdrFfiError::Ok,
                        Err(err) => err,
                    }
                }
            }
        }
    };
}

pub(crate) use impl_ffi_deserialize_for;
pub(crate) use impl_ffi_serialize_for;
