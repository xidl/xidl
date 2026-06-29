#![allow(unused)]
#![allow(clippy::too_many_arguments)]
#![allow(nonstandard_style)]
mod api {
    include!(concat!(env!("OUT_DIR"), "/reddit.rs"));
}

pub use api::*;
