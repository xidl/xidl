#![allow(dead_code)]

mod emscripten;
#[cfg(target_os = "emscripten")]
pub use emscripten::*;

#[cfg(all(not(target_os = "emscripten"), unix))]
mod fallback;
#[cfg(all(not(target_os = "emscripten"), unix))]
pub use fallback::*;

#[cfg(all(not(target_os = "emscripten"), windows))]
mod windows_pipe;
#[cfg(all(not(target_os = "emscripten"), windows))]
pub use windows_pipe::*;
