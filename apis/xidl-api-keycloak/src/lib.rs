mod api {
    include!(concat!(env!("OUT_DIR"), "/keycloak.rs"));
}

pub use api::*;
