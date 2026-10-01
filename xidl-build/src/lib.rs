//! Build-script support for generating code from XIDL interface files.
//!
//! By default this crate shells out to the `xidlc` executable found on
//! `PATH`, so downstream builds carry no compiler dependency. The `bundle`
//! feature links the `xidlc` compiler in-process with the default
//! generator set; `gen-*` features add more generators to bundled builds.
//!
//! # Example
//!
//! ```no_run
//! fn main() -> Result<(), xidl_build::Error> {
//!     println!("cargo:rerun-if-changed=idl/hello_world.idl");
//!
//!     xidl_build::Builder::new()
//!         .with_lang("rust")
//!         .compile(&["idl/hello_world.idl"])?;
//!
//!     Ok(())
//! }
//! ```

mod builder;
mod error;

#[cfg(not(feature = "bundle"))]
mod cli;

#[cfg(feature = "bundle")]
mod bundle;

pub use builder::Builder;
pub use error::Error;
