#![allow(deprecated)]

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
