#[path = "xcdr_support.rs"]
mod xcdr_support;

pub(crate) use xcdr_support::{
    declarator_info, element_kind, kind_json, switch_kind, type_kind, type_kind_from_c,
};
