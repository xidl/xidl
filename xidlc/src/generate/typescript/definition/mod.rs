pub(crate) mod annotations;
mod attr;
pub(crate) mod contexts;
mod http;
mod interface_render;
mod method;
mod module_render;
pub(crate) mod names;
mod operation;
mod output;
mod root_imports;
mod route_template;
mod stream_validation;
mod struct_fields;
pub(crate) mod type_expr;
mod type_render;
mod type_render_helpers;

#[cfg(test)]
mod tests;

#[cfg(any(
    feature = "gen-typescript-rest",
    feature = "gen-typescript-rest-adapters"
))]
pub(crate) use method::TypeRefTarget;
pub use module_render::render_typescript;
