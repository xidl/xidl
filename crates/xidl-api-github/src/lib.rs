#![allow(unused)]
#[allow(clippy::too_many_arguments)]
mod api {
    include!(concat!(env!("OUT_DIR"), "/github.rs"));
}

pub use api::*;
