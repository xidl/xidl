mod cdr;
mod delimited_cdr;
mod macros;
mod plain_cdr2;
mod plcdr;
mod plcdr2;
mod xcdr_plcdr;

use crate::error::{XcdrError, XcdrResult};

#[repr(C)]
pub enum XcdrFfiError {
    Ok = 0,
    BufferOverflow = 1,
    Message = 2,
    NullPointer = 3,
}

#[repr(C)]
pub struct XcdrBuffer {
    pub ptr: *mut u8,
    pub len: usize,
    pub pos: usize,
}

#[repr(C)]
pub struct XcdrConstBuffer {
    pub ptr: *const u8,
    pub len: usize,
    pub pos: usize,
}

#[repr(C)]
pub struct XcdrBufferResult {
    pub err: XcdrFfiError,
    pub used: usize,
}

impl From<XcdrError> for XcdrFfiError {
    fn from(err: XcdrError) -> Self {
        match err {
            XcdrError::BufferOverflow => Self::BufferOverflow,
            XcdrError::Message(_) => Self::Message,
        }
    }
}

impl From<XcdrResult<()>> for XcdrFfiError {
    fn from(err: XcdrResult<()>) -> Self {
        match err {
            Ok(_) => Self::Ok,
            Err(err) => err.into(),
        }
    }
}
