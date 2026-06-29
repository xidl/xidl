#![allow(unused)]
#[allow(clippy::too_many_arguments)]
mod api {
    include!(concat!(env!("OUT_DIR"), "/discord.rs"));
}

pub use api::*;
