//! Implements [OpenAPI Metadata][info] types.
//!
//! [info]: <https://spec.openapis.org/oas/latest.html#info-object>

mod contact;
mod info_object;
mod license;
#[cfg(test)]
mod tests;

pub use self::{
    contact::{Contact, ContactBuilder},
    info_object::{Info, InfoBuilder},
    license::{License, LicenseBuilder},
};
