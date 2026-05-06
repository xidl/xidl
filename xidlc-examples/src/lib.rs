#![allow(unused_mut)]
#![allow(non_snake_case)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(private_interfaces)]
#![allow(dead_code)]
#![allow(unused)]
#![allow(clippy::redundant_field_names)]
#![allow(clippy::map_identity)]

pub mod hello_world {
    include!(concat!(env!("OUT_DIR"), "/hello_world.rs"));
}

pub mod hello_world_jsonrpc {
    include!(concat!(env!("OUT_DIR"), "/hello_world_jsonrpc.rs"));
}

pub mod city_rest;
pub mod city_rest_stream;

pub mod city_jsonrpc;
pub mod city_jsonrpc_stream;

pub mod hysteria2;
pub mod rest_media_types;
pub mod rest_server;
pub mod union_serde;

pub mod e2e_test;
