use crate::error::IdlcResult;
use crate::generate::typescript::TypescriptRenderer;
use serde::Serialize;
use xidl_parser::hir;
use xidl_parser::rest_hir::RestHirDocument;

use super::interface::render_interface;
use super::model::TsHttpBlocks;

#[derive(Serialize)]
struct TypesFileContext {
    blocks: Vec<String>,
}

#[derive(Serialize)]
struct ClientFileContext {
    file_stem: String,
    helpers: Vec<String>,
    blocks: Vec<String>,
}

#[derive(Serialize)]
struct ServerFileContext {
    file_stem: String,
    helpers: Vec<String>,
    blocks: Vec<String>,
}

#[derive(Serialize)]
struct ModuleContext {
    ident: String,
    blocks: Vec<String>,
}

pub(crate) struct TsHttpOutput {
    pub(crate) types: String,
    pub(crate) zod: String,
    pub(crate) client: String,
    pub(crate) server: String,
}

pub(crate) fn render_spec(
    spec: &hir::Specification,
    file_stem: &str,
    renderer: &TypescriptRenderer,
    rest_hir: &RestHirDocument,
) -> IdlcResult<TsHttpOutput> {
    let blocks = render_defs(&spec.0, &[], renderer, rest_hir)?;
    Ok(TsHttpOutput {
        types: renderer.render_template(
            "http/types.d.ts.j2",
            &TypesFileContext {
                blocks: blocks.types,
            },
        )?,
        zod: renderer
            .render_template("http/zod.ts.j2", &TypesFileContext { blocks: blocks.zod })?,
        client: renderer.render_template(
            "http/client.ts.j2",
            &ClientFileContext {
                file_stem: file_stem.to_string(),
                helpers: vec![renderer.render_template("http/client_helpers.ts.j2", &())?],
                blocks: blocks.client,
            },
        )?,
        server: renderer.render_template(
            "http/server.ts.j2",
            &ServerFileContext {
                file_stem: file_stem.to_string(),
                helpers: vec![renderer.render_template("http/server_helpers.ts.j2", &())?],
                blocks: blocks.server,
            },
        )?,
    })
}

fn render_defs(
    defs: &[hir::Definition],
    module_path: &[String],
    renderer: &TypescriptRenderer,
    rest_hir: &RestHirDocument,
) -> IdlcResult<TsHttpBlocks> {
    let mut out = TsHttpBlocks::default();
    for def in defs {
        match def {
            hir::Definition::ModuleDcl(module) => {
                let mut next = module_path.to_vec();
                next.push(module.ident.clone());
                let body = render_defs(&module.definition, &next, renderer, rest_hir)?;
                if !body.is_empty() {
                    let ident =
                        crate::generate::typescript::definition::names::ts_ident(&module.ident);
                    out.types
                        .push(render_module(renderer, &ident, &body.types.join("\n"))?);
                    out.zod
                        .push(render_module(renderer, &ident, &body.zod.join("\n"))?);
                    out.client
                        .push(render_module(renderer, &ident, &body.client.join("\n"))?);
                    out.server
                        .push(render_module(renderer, &ident, &body.server.join("\n"))?);
                }
            }
            hir::Definition::InterfaceDcl(interface) => {
                out.extend(render_interface(
                    interface,
                    module_path,
                    renderer,
                    rest_hir,
                )?);
            }
            _ => {}
        }
    }
    Ok(out)
}

fn render_module(renderer: &TypescriptRenderer, ident: &str, body: &str) -> IdlcResult<String> {
    renderer.render_template(
        "http/module.ts.j2",
        &ModuleContext {
            ident: ident.to_string(),
            blocks: vec![crate::generate::typescript::definition::names::indent_block(body, 1)],
        },
    )
}
