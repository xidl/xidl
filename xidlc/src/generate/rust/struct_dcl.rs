use crate::error::IdlcResult;
use crate::generate::rust::util::{
    declarator_name, rust_scoped_name, serialize_kind_name, type_with_decl,
};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::StructForwardDcl {
    fn render(&self, _renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        Ok(RustRenderOutput::default())
    }
}

impl RustRender for hir::StructDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        render_struct_with_config(self, renderer, &hir::SerializeConfig::default(), &[], &[])
    }
}

pub(crate) fn render_struct_with_config(
    def: &hir::StructDcl,
    renderer: &RustRenderer,
    config: &hir::SerializeConfig,
    module_path: &[String],
    annotations: &[hir::Annotation],
) -> IdlcResult<RustRenderOutput> {
    let parent = def.parent.first().map(rust_scoped_name);
    let members = def
        .member
        .iter()
        .flat_map(|member| {
            let field_id = member.field_id.map(|value| format!("{value}u32"));
            member.ident.iter().map(move |decl| {
                let name = crate::generate::rust::util::rust_ident(&declarator_name(decl));
                let ty = type_with_decl(&member.ty, decl);
                json!({ "ty": ty, "name": name, "field_id": field_id.clone() })
            })
        })
        .collect::<Vec<_>>();
    let serialize_kind = serialize_kind_name(def.serialize_kind(config));
    let derive = crate::generate::rust::util::rust_derives_from_annotations_with_extra(
        &def.annotations,
        annotations,
    );
    let module_path = module_path.to_vec();
    let ctx = json!({
        "ident": crate::generate::rust::util::rust_ident(&def.ident),
        "parent": parent,
        "members": members,
        "serialize_kind": serialize_kind,
        "derive": derive,
        "module_path": module_path,
        "typeobject_path": renderer.typeobject_path(),
    });
    let rendered = renderer.render_template("struct.rs.j2", &ctx)?;
    Ok(RustRenderOutput::default().push_source(rendered))
}
