use crate::error::IdlcResult;
use crate::generate::typescript::TypescriptRenderer;
use crate::generate::utils::doc_lines_from_annotations;
use xidl_parser::hir;

use super::contexts::{
    EnumTypeContext, EnumZodContext, FieldTypeContext, FieldZodContext, StructTypeContext,
    StructZodContext, TypedefTypeContext, TypedefZodContext, UnionTypeContext, UnionZodContext,
};
use super::method::TypeRefTarget;
use super::names::{declarator_name, ts_ident, ts_prop_name};
use super::output::TsRenderOutput;
use super::struct_fields::struct_fields;
use super::type_expr::{
    apply_array_ts, apply_array_zod, ts_type_for_constr_inline, ts_type_for_decl,
    ts_type_for_element, zod_schema_for_constr_inline, zod_schema_for_decl, zod_schema_for_element,
};
use super::type_render_helpers::{render_bit_number, render_native};

pub(crate) fn render_constr_type(
    constr: &hir::ConstrTypeDcl,
    module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<TsRenderOutput> {
    let mut out = TsRenderOutput::default();
    match constr {
        hir::ConstrTypeDcl::StructDcl(def) => {
            out.extend(render_struct(def, module_path, renderer)?)
        }
        hir::ConstrTypeDcl::EnumDcl(def) => out.extend(render_enum(def, renderer)?),
        hir::ConstrTypeDcl::UnionDef(def) => out.extend(render_union(def, module_path, renderer)?),
        hir::ConstrTypeDcl::BitsetDcl(def) => out.extend(render_bit_number(&def.ident, renderer)?),
        hir::ConstrTypeDcl::BitmaskDcl(def) => out.extend(render_bit_number(&def.ident, renderer)?),
        hir::ConstrTypeDcl::StructForwardDcl(_) | hir::ConstrTypeDcl::UnionForwardDcl(_) => {}
    }
    Ok(out)
}

pub(crate) fn render_exception(
    except: &hir::ExceptDcl,
    module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<TsRenderOutput> {
    render_struct(
        &hir::StructDcl {
            annotations: Vec::new(),
            ident: except.ident.clone(),
            parent: Vec::new(),
            member: except.member.clone(),
        },
        module_path,
        renderer,
    )
}

pub(crate) fn render_type_dcl(
    ty: &hir::TypeDcl,
    module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<TsRenderOutput> {
    let mut out = TsRenderOutput::default();
    match ty {
        hir::TypeDcl::ConstrTypeDcl(constr) => {
            out.extend(render_constr_type(constr, module_path, renderer)?)
        }
        hir::TypeDcl::TypedefDcl(typedef) => {
            render_typedefs(&mut out, typedef, module_path, renderer)?
        }
        hir::TypeDcl::NativeDcl(native) => render_native(&mut out, native, renderer)?,
    }
    Ok(out)
}

fn render_typedefs(
    out: &mut TsRenderOutput,
    typedef: &hir::TypedefDcl,
    module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<()> {
    for decl in &typedef.decl {
        let name = ts_ident(declarator_name(decl));
        let type_expr = match &typedef.ty {
            hir::TypedefType::TypeSpec(spec) => {
                ts_type_for_decl(spec, decl, module_path, TypeRefTarget::Types)
            }
            hir::TypedefType::ConstrTypeDcl(constr) => {
                let base = ts_type_for_constr_inline(constr, module_path, TypeRefTarget::Types);
                apply_array_ts(base, decl)
            }
        };
        out.types.push(renderer.render_template(
            "typedef.d.ts.j2",
            &TypedefTypeContext {
                name: name.clone(),
                type_expr,
                doc: doc_lines_from_annotations(&typedef.annotations),
            },
        )?);
        let schema_expr = match &typedef.ty {
            hir::TypedefType::TypeSpec(spec) => zod_schema_for_decl(spec, decl, module_path),
            hir::TypedefType::ConstrTypeDcl(constr) => {
                let base = zod_schema_for_constr_inline(constr, module_path);
                apply_array_zod(base, decl)
            }
        };
        out.zod.push(renderer.render_template(
            "typedef.zod.ts.j2",
            &TypedefZodContext {
                schema_name: format!("{name}Schema"),
                name,
                schema_expr,
            },
        )?);
    }
    Ok(())
}

fn render_struct(
    def: &hir::StructDcl,
    module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<TsRenderOutput> {
    let ident = ts_ident(&def.ident);
    let extends = {
        let parents = def
            .parent
            .iter()
            .map(|parent| super::names::ts_scoped_name(parent, module_path, TypeRefTarget::Types))
            .collect::<Vec<_>>();
        (!parents.is_empty()).then(|| parents.join(", "))
    };
    let fields = struct_fields(&def.member, &def.annotations, module_path);
    let types = renderer.render_template(
        "struct.d.ts.j2",
        &StructTypeContext {
            ident: ident.clone(),
            extends,
            doc: doc_lines_from_annotations(&def.annotations),
            fields: fields
                .iter()
                .map(|field| FieldTypeContext {
                    prop: field.prop.clone(),
                    ty: field.ty.clone(),
                    optional: field.optional,
                    doc: field.doc.clone(),
                })
                .collect(),
        },
    )?;
    let zod = renderer.render_template(
        "struct.zod.ts.j2",
        &StructZodContext {
            schema_name: format!("{ident}Schema"),
            ident,
            fields: fields
                .into_iter()
                .map(|field| FieldZodContext {
                    prop: field.prop,
                    schema: field.schema,
                    optional: field.optional,
                    xjson_meta: field.xjson_meta,
                })
                .collect(),
        },
    )?;
    Ok(TsRenderOutput {
        types: vec![types],
        zod: vec![zod],
        client: Vec::new(),
    })
}

fn render_enum(def: &hir::EnumDcl, renderer: &TypescriptRenderer) -> IdlcResult<TsRenderOutput> {
    let ident = ts_ident(&def.ident);
    let members = def
        .member
        .iter()
        .map(|value| {
            format!(
                "\"{}\"",
                hir::effective_wire_name(&value.ident, &value.annotations, &def.annotations)
            )
        })
        .collect::<Vec<_>>();
    Ok(TsRenderOutput {
        types: vec![renderer.render_template(
            "enum.d.ts.j2",
            &EnumTypeContext {
                ident: ident.clone(),
                union: if members.is_empty() {
                    "never".into()
                } else {
                    members.join(" | ")
                },
                doc: doc_lines_from_annotations(&def.annotations),
            },
        )?],
        zod: vec![renderer.render_template(
            "enum.zod.ts.j2",
            &EnumZodContext {
                schema_name: format!("{ident}Schema"),
                ident,
                values: members,
            },
        )?],
        client: Vec::new(),
    })
}

fn render_union(
    def: &hir::UnionDef,
    module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<TsRenderOutput> {
    let ident = ts_ident(&def.ident);
    let variants = def
        .case
        .iter()
        .map(|case| {
            let prop = ts_prop_name(declarator_name(&case.element.value));
            let ty = ts_type_for_element(
                &case.element.ty,
                &case.element.value,
                module_path,
                TypeRefTarget::Types,
            );
            format!("{{ {prop}: {ty} }}")
        })
        .collect::<Vec<_>>();
    let schema_variants = def
        .case
        .iter()
        .map(|case| {
            let prop = ts_prop_name(declarator_name(&case.element.value));
            let schema = zod_schema_for_element(&case.element.ty, &case.element.value, module_path);
            format!("z.object({{ {prop}: {schema} }})")
        })
        .collect::<Vec<_>>();
    Ok(TsRenderOutput {
        types: vec![renderer.render_template(
            "union.d.ts.j2",
            &UnionTypeContext {
                ident: ident.clone(),
                union: if variants.is_empty() {
                    "never".into()
                } else {
                    variants.join(" | ")
                },
                doc: doc_lines_from_annotations(&def.annotations),
            },
        )?],
        zod: vec![renderer.render_template(
            "union.zod.ts.j2",
            &UnionZodContext {
                schema_name: format!("{ident}Schema"),
                ident,
                variants: schema_variants,
            },
        )?],
        client: Vec::new(),
    })
}
