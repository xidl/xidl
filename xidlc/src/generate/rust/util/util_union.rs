//! Names used by ordinary Rust unions and their independent adapters.

use convert_case::{Case, Casing};
use xidl_parser::hir;

use crate::generate::rust::util::{render_const, rust_ident};
use crate::generate::utils::scope::relative_path;

/// Rust variant identifier for a built-in HTTP content type.
pub fn http_union_case_ident(ident: &str) -> String {
    ident
        .strip_prefix("r#")
        .unwrap_or(ident)
        .to_case(Case::Pascal)
}

/// Wire discriminator spelling used by the ordinary union's Serde layout.
pub fn union_serde_tag_string(label: &str) -> String {
    label.rsplit("::").next().unwrap_or(label).to_string()
}

/// Rust expression for one ordinary union case label. A label naming an
/// enumerator of the switch enum — bare, or spelled through the enum — renders
/// as the enum path resolved from the rendering module followed by the
/// enumerator, so the generated code compiles however the IDL spelled it.
/// Constants of the enum type and every other expression render as written.
pub fn union_case_label(
    expr: &hir::ConstExpr,
    switch_enum: Option<&hir::SwitchEnum>,
    module_path: &[String],
) -> String {
    let Some(switch_enum) = switch_enum else {
        return render_const(expr);
    };
    let hir::ConstExpr::ScopedName(name) = expr else {
        return render_const(expr);
    };
    let Some(enumerator) = switch_enum.enumerator_named(name) else {
        return render_const(expr);
    };
    format!(
        "{}::{}",
        relative_path(module_path, &switch_enum.path.join("::")),
        rust_ident(enumerator),
    )
}
