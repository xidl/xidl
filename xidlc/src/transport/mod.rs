#![allow(dead_code)]

mod emscripten;
#[cfg(target_os = "emscripten")]
pub use emscripten::*;

#[cfg(not(target_os = "emscripten"))]
mod fallback;
#[cfg(not(target_os = "emscripten"))]
pub use fallback::*;

mod mux_listener;
pub(crate) use mux_listener::MuxListener;
