use crate::error::IdlcResult;
use crate::generate::typescript::{TsMode, TypescriptRenderOutput, TypescriptRenderer};
use xidl_parser::hir;

use super::contexts::{ClientFileContext, ModuleContext, TypesFileContext};
use super::interface_render::render_interface;
use super::names::{indent_block, ts_ident};
use super::output::TsRenderOutput;
use super::type_render::{render_constr_type, render_exception, render_type_dcl};

pub fn render_typescript(
    spec: &hir::Specification,
    file_stem: &str,
    renderer: &TypescriptRenderer,
    mode: TsMode,
) -> IdlcResult<TypescriptRenderOutput> {
    let mut generator = TsGenerator::new(file_stem.to_string());
    generator.render(spec, renderer, mode)
}

struct TsGenerator {
    file_stem: String,
}

impl TsGenerator {
    fn new(file_stem: String) -> Self {
        Self { file_stem }
    }

    fn render(
        &mut self,
        spec: &hir::Specification,
        renderer: &TypescriptRenderer,
        mode: TsMode,
    ) -> IdlcResult<TypescriptRenderOutput> {
        let blocks = render_module_body(&spec.0, &[], renderer, mode)?;
        let types = renderer.render_template(
            "types.d.ts.j2",
            &TypesFileContext {
                blocks: blocks.types,
            },
        )?;
        let zod =
            renderer.render_template("zod.ts.j2", &TypesFileContext { blocks: blocks.zod })?;
        let client = if mode.allows_interfaces() {
            let helpers = renderer.render_template("client_helpers.ts.j2", &())?;
            renderer.render_template(
                "client.ts.j2",
                &ClientFileContext {
                    file_stem: self.file_stem.clone(),
                    helpers: vec![helpers],
                    blocks: blocks.client,
                },
            )?
        } else {
            String::new()
        };
        Ok(TypescriptRenderOutput { types, zod, client })
    }
}

pub(crate) fn render_module_body(
    defs: &[hir::Definition],
    module_path: &[String],
    renderer: &TypescriptRenderer,
    mode: TsMode,
) -> IdlcResult<TsRenderOutput> {
    let mut out = TsRenderOutput::default();
    let mut module_order = Vec::new();
    let mut module_map = std::collections::HashMap::<String, Vec<TsRenderOutput>>::new();

    for def in defs {
        match def {
            hir::Definition::ModuleDcl(module) => {
                let mut next_path = module_path.to_vec();
                next_path.push(module.ident.clone());
                let body = render_module_body(&module.definition, &next_path, renderer, mode)?;
                if !body.is_empty() {
                    let entry = module_map.entry(module.ident.clone()).or_insert_with(|| {
                        module_order.push(module.ident.clone());
                        Vec::new()
                    });
                    entry.push(body);
                }
            }
            hir::Definition::ConstrTypeDcl(constr) if mode.allows_types() => {
                out.extend(render_constr_type(constr, module_path, renderer)?);
            }
            hir::Definition::TypeDcl(ty) if mode.allows_types() => {
                out.extend(render_type_dcl(ty, module_path, renderer)?);
            }
            hir::Definition::ExceptDcl(except) if mode.allows_types() => {
                out.extend(render_exception(except, module_path, renderer)?);
            }
            hir::Definition::InterfaceDcl(interface) if mode.allows_interfaces() => {
                out.extend(render_interface(interface, module_path, renderer)?);
            }
            hir::Definition::ConstDcl(_) | hir::Definition::Pragma(_) => {}
            _ => {}
        }
    }

    for name in module_order {
        let body = merge_blocks(&module_map.remove(&name).unwrap_or_default());
        let ident = ts_ident(&name);
        out.types.push(render_module_block(
            renderer,
            ident.clone(),
            body.types.join("\n"),
        )?);
        out.zod.push(render_module_block(
            renderer,
            ident.clone(),
            body.zod.join("\n"),
        )?);
        out.client.push(render_module_block(
            renderer,
            ident,
            body.client.join("\n"),
        )?);
    }

    Ok(out)
}

fn render_module_block(
    renderer: &TypescriptRenderer,
    ident: String,
    body: String,
) -> IdlcResult<String> {
    renderer.render_template(
        "module.ts.j2",
        &ModuleContext {
            ident,
            body: indent_block(&body, 1),
        },
    )
}

fn merge_blocks(blocks: &[TsRenderOutput]) -> TsRenderOutput {
    let mut out = TsRenderOutput::default();
    for block in blocks {
        out.types.extend(block.types.iter().cloned());
        out.zod.extend(block.zod.iter().cloned());
        out.client.extend(block.client.iter().cloned());
    }
    out
}
