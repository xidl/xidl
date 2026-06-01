use crate::error::IdlcResult;
use crate::generate::typescript::TypescriptRenderer;
use serde::Serialize;
use xidl_parser::hir;
use xidl_parser::rest_hir::RestHirDocument;

use super::interface::render_interface;
use super::model::TsHttpBlocks;

#[derive(Serialize)]
struct TypesFileContext {
    file_stem: String,
    imports: Vec<String>,
    blocks: Vec<String>,
}

#[derive(Serialize)]
struct ZodFileContext {
    file_stem: String,
    imports: Vec<String>,
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
    let model_exports = ZodImportCollector::collect_exported_names(spec);
    let mut zod_imports = Vec::new();
    let mut type_imports = Vec::new();
    for name in model_exports {
        let schema_name = format!("{name}Schema");
        let has_schema = blocks
            .zod
            .iter()
            .any(|block| ZodImportCollector::is_word_in_text(&schema_name, block));
        let has_mod_zod = blocks
            .zod
            .iter()
            .any(|block| ZodImportCollector::is_word_in_text(&name, block));
        if has_schema {
            zod_imports.push(schema_name);
        } else if has_mod_zod {
            zod_imports.push(name.clone());
        }

        if blocks
            .types
            .iter()
            .any(|block| ZodImportCollector::is_word_in_text(&name, block))
        {
            type_imports.push(name);
        }
    }
    Ok(TsHttpOutput {
        types: renderer.render_template(
            "http/types.d.ts.j2",
            &TypesFileContext {
                file_stem: file_stem.to_string(),
                imports: type_imports,
                blocks: blocks.types,
            },
        )?,
        zod: renderer.render_template(
            "http/zod.ts.j2",
            &ZodFileContext {
                file_stem: file_stem.to_string(),
                imports: zod_imports,
                blocks: blocks.zod,
            },
        )?,
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

struct ZodImportCollector;

impl ZodImportCollector {
    fn collect_exported_names(spec: &hir::Specification) -> Vec<String> {
        let mut names = std::collections::BTreeSet::new();
        for def in &spec.0 {
            match def {
                hir::Definition::ModuleDcl(module) => {
                    names.insert(crate::generate::typescript::definition::names::ts_ident(
                        &module.ident,
                    ));
                }
                hir::Definition::ConstrTypeDcl(constr) => {
                    Self::collect_constr_names(constr, &mut names);
                }
                hir::Definition::TypeDcl(ty) => match ty {
                    hir::TypeDcl::ConstrTypeDcl(constr) => {
                        Self::collect_constr_names(constr, &mut names);
                    }
                    hir::TypeDcl::TypedefDcl(typedef) => {
                        for decl in &typedef.decl {
                            let name = crate::generate::typescript::definition::names::ts_ident(
                                crate::generate::typescript::definition::names::declarator_name(
                                    decl,
                                ),
                            );
                            names.insert(name);
                        }
                    }
                    hir::TypeDcl::NativeDcl(native) => {
                        let name = crate::generate::typescript::definition::names::ts_ident(
                            &native.decl.0,
                        );
                        names.insert(name);
                    }
                },
                hir::Definition::ExceptDcl(except) => {
                    let name =
                        crate::generate::typescript::definition::names::ts_ident(&except.ident);
                    names.insert(name);
                }
                _ => {}
            }
        }
        names.into_iter().collect()
    }

    fn collect_constr_names(
        constr: &hir::ConstrTypeDcl,
        names: &mut std::collections::BTreeSet<String>,
    ) {
        match constr {
            hir::ConstrTypeDcl::StructDcl(def) => {
                let name = crate::generate::typescript::definition::names::ts_ident(&def.ident);
                names.insert(name);
            }
            hir::ConstrTypeDcl::EnumDcl(def) => {
                let name = crate::generate::typescript::definition::names::ts_ident(&def.ident);
                names.insert(name);
            }
            hir::ConstrTypeDcl::UnionDef(def) => {
                let name = crate::generate::typescript::definition::names::ts_ident(&def.ident);
                names.insert(name);
            }
            hir::ConstrTypeDcl::BitsetDcl(def) => {
                let name = crate::generate::typescript::definition::names::ts_ident(&def.ident);
                names.insert(name);
            }
            hir::ConstrTypeDcl::BitmaskDcl(def) => {
                let name = crate::generate::typescript::definition::names::ts_ident(&def.ident);
                names.insert(name);
            }
            _ => {}
        }
    }

    fn is_word_in_text(word: &str, text: &str) -> bool {
        let mut start = 0;
        while let Some(idx) = text[start..].find(word) {
            let match_start = start + idx;
            let match_end = match_start + word.len();
            let before_ok = match_start == 0 || {
                text[..match_start]
                    .chars()
                    .next_back()
                    .map(|c| !c.is_ascii_alphanumeric() && c != '_')
                    .unwrap_or(true)
            };
            let after_ok = match_end == text.len() || {
                text[match_end..]
                    .chars()
                    .next()
                    .map(|c| !c.is_ascii_alphanumeric() && c != '_')
                    .unwrap_or(true)
            };
            if before_ok && after_ok {
                return true;
            }
            start = match_start + 1;
        }
        false
    }
}
