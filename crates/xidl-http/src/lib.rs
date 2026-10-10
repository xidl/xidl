//! HTTP protocol definitions shared by the compiler and transport runtimes.
//!
//! This crate has no parser, network, or async runtime dependencies.

mod content_type;

pub use content_type::ContentType;
