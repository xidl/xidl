#![allow(unused_mut)]
#![allow(non_snake_case)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(private_interfaces)]

pub mod hello_world {
    include!(concat!(env!("OUT_DIR"), "/hello_world.rs"));
}

pub mod hello_world_jsonrpc {
    include!(concat!(env!("OUT_DIR"), "/hello_world_jsonrpc.rs"));
}

pub mod city_http {
    include!(concat!(env!("OUT_DIR"), "/city_http.rs"));
}

pub mod city_http_stream {
    include!(concat!(env!("OUT_DIR"), "/city_http_stream.rs"));
}

pub mod city_jsonrpc;
pub mod city_jsonrpc_stream;

pub mod example_services;
