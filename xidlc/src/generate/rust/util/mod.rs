mod util_annotation_parse;
mod util_annotations;
mod util_declarators;
mod util_derives;
mod util_types;

#[allow(unused_imports)]
pub use self::util_annotations::rust_passthrough_attrs_from_annotations;
#[allow(unused_imports)]
pub use self::util_declarators::{
    array_type, constr_type_scoped_name, declarator_dims, declarator_name, member_json,
    type_with_decl, typedef_json,
};
#[allow(unused_imports)]
pub use self::util_derives::{
    RustDeriveInfo, rust_derive_info_with_extra, rust_derives_from_annotations,
    rust_derives_from_annotations_with_extra,
};
#[allow(unused_imports)]
pub use self::util_types::{
    bitfield_type, render_const, rust_const_type, rust_ident, rust_integer_type, rust_literal,
    rust_scoped_name, rust_switch_type, rust_type,
};
#[allow(unused_imports)]
pub use xidl_parser::hir::{
    field_rename as serde_rename_from_annotations, is_skipped as is_skipped_from_annotations,
    rename_all as serde_rename_all_from_annotations,
};
