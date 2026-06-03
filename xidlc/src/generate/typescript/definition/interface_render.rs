use crate::error::IdlcResult;
use crate::generate::typescript::TypescriptRenderer;
use xidl_parser::hir;

use super::contexts::{
    ClientClassContext, ParamDeclContext, RequestContext, RequestZodContext, TsType, ZodSchema,
};
use super::method::{MethodInfo, TypeRefTarget};
use super::names::{ts_ident, ts_prop_name};
use super::operation::render_op;
use super::output::TsRenderOutput;
use super::type_expr::{ts_type_for_type_spec, zod_schema_for_type_spec};

pub(crate) fn render_interface(
    interface: &hir::InterfaceDcl,
    module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<TsRenderOutput> {
    let def = match &interface.decl {
        hir::InterfaceDclInner::InterfaceDef(def) => def,
        _ => return Ok(TsRenderOutput::default()),
    };
    let methods = collect_methods(def, module_path)?;
    let mut out = TsRenderOutput::default();
    for method in &methods {
        render_request(out_ref(&mut out), method, module_path, renderer)?;
        render_response(out_ref(&mut out), method, module_path, renderer)?;
    }
    out.client.push(
        renderer.render_template(
            "client_class.ts.j2",
            &ClientClassContext {
                client_name: ts_ident(&def.header.ident), // Template adds 'Client'
                methods: methods
                    .iter()
                    .map(|method| method.to_template(module_path))
                    .collect(),
            },
        )?,
    );
    Ok(out)
}

fn collect_methods(def: &hir::InterfaceDef, module_path: &[String]) -> IdlcResult<Vec<MethodInfo>> {
    let mut methods = Vec::new();
    if let Some(body) = &def.interface_body {
        for export in &body.0 {
            match export {
                hir::Export::OpDcl(op) => {
                    methods.push(render_op(op, &def.header.ident, module_path)?)
                }
                hir::Export::AttrDcl(attr) => {
                    methods.extend(super::attr::render_attr(
                        attr,
                        &def.header.ident,
                        module_path,
                    ));
                }
                _ => {}
            }
        }
    }
    Ok(methods)
}

fn render_request(
    out: &mut TsRenderOutput,
    method: &MethodInfo,
    module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<()> {
    let Some(request_name) = &method.request_name else {
        return Ok(());
    };
    let params = method
        .params
        .iter()
        .map(|param| ParamDeclContext {
            prop: ts_prop_name(&param.raw_name),
            ty: ts_type_for_type_spec(&param.ty, module_path, TypeRefTarget::Types),
            schema: zod_schema_for_type_spec(&param.ty, module_path),
            optional: param.optional,
            doc: param.doc.clone(),
        })
        .collect::<Vec<_>>();
    out.types.push(renderer.render_template(
        "request.d.ts.j2",
        &RequestContext {
            name: request_name.clone(),
            params: params.clone(),
            doc: method.doc.clone(),
        },
    )?);
    out.zod.push(renderer.render_template(
        "request.zod.ts.j2",
        &RequestZodContext {
            schema_name: request_name.clone(), // Template adds 'Schema'
            name: request_name.clone(),
            params,
        },
    )?);
    Ok(())
}

fn render_response(
    out: &mut TsRenderOutput,
    method: &MethodInfo,
    module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<()> {
    let Some(response_name) = &method.response_name else {
        return Ok(());
    };
    let mut params = vec![ParamDeclContext {
        prop: ts_prop_name("return"),
        ty: response_type(method, module_path),
        schema: response_schema(method, module_path),
        optional: false,
        doc: Vec::new(),
    }];
    params.extend(method.output_params.iter().map(|param| ParamDeclContext {
        prop: ts_prop_name(&param.raw_name),
        ty: ts_type_for_type_spec(&param.ty, module_path, TypeRefTarget::Types),
        schema: zod_schema_for_type_spec(&param.ty, module_path),
        optional: param.optional,
        doc: param.doc.clone(),
    }));
    out.types.push(renderer.render_template(
        "request.d.ts.j2",
        &RequestContext {
            name: response_name.clone(),
            params: params.clone(),
            doc: method.doc.clone(),
        },
    )?);
    out.zod.push(renderer.render_template(
        "request.zod.ts.j2",
        &RequestZodContext {
            schema_name: response_name.clone(), // Template adds 'Schema'
            name: response_name.clone(),
            params,
        },
    )?);
    Ok(())
}

fn response_type(method: &MethodInfo, module_path: &[String]) -> TsType {
    if method.ret.is_void {
        TsType::Void
    } else {
        ts_type_for_type_spec(
            method.ret.ty.as_ref().expect("return type"),
            module_path,
            TypeRefTarget::Types,
        )
    }
}

fn response_schema(method: &MethodInfo, module_path: &[String]) -> ZodSchema {
    if method.ret.is_void {
        ZodSchema::Primitive("void()".to_string())
    } else {
        zod_schema_for_type_spec(method.ret.ty.as_ref().expect("return type"), module_path)
    }
}

fn out_ref(out: &mut TsRenderOutput) -> &mut TsRenderOutput {
    out
}
