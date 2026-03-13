#![allow(dead_code)]

#[cfg(unix)]
mod fallback;
#[cfg(unix)]
pub use fallback::*;

#[cfg(windows)]
mod windows_pipe;
#[cfg(windows)]
pub use windows_pipe::*;
