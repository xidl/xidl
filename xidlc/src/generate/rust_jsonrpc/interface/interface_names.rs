use crate::generate::rust::util::rust_ident;
use convert_case::{Case, Casing};

pub(super) fn response_struct_name(interface_name: &str, method_name: &str) -> String {
    let method_name = method_name.strip_prefix("r#").unwrap_or(method_name);
    format!(
        "{}{}Result",
        rust_ident(interface_name),
        method_name.to_case(Case::Camel)
    )
}

pub(super) fn params_struct_name(interface_name: &str, method_name: &str) -> String {
    let method_name = method_name.strip_prefix("r#").unwrap_or(method_name);
    format!(
        "{}{}Params",
        rust_ident(interface_name),
        method_name.to_case(Case::Camel)
    )
}

pub(super) fn rpc_method_name(
    module_path: &[String],
    interface_name: &str,
    method_name: &str,
) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    parts.join(".")
}
