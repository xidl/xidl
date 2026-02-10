#![allow(unused_mut)]
#![allow(non_snake_case)]
#![allow(unused_imports)]

pub mod hello_world {
    include!(concat!(env!("OUT_DIR"), "/hello_world.rs"));
}

pub mod hello_world_jsonrpc {
    include!(concat!(env!("OUT_DIR"), "/hello_world_jsonrpc.rs"));
}
