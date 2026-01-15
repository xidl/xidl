mod cdr;
mod macros;
mod plain_cdr2;
mod plcdr;
mod xcdr_plcdr;

use crate::error::{XcdrError, XcdrResult};

#[repr(C)]
pub enum XcdrFfiError {
    Ok = 0,
    BufferOverflow = 1,
    Message = 2,
    NullPointer = 3,
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
