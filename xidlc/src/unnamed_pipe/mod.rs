mod emscripten;
#[cfg(target_os = "emscripten")]
pub use emscripten::*;

#[cfg(not(target_os = "emscripten"))]
mod fallback;
#[cfg(not(target_os = "emscripten"))]
pub use fallback::*;
