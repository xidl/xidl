use crate::error::IdlcResult;
use crate::generate::typescript::TypescriptRenderer;
use crate::generate::typescript::definition::TypeRefTarget;
use crate::generate::typescript::definition::contexts::TsType;
use crate::generate::typescript::definition::type_expr::ts_type_for_type_spec;
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
    blocks: Vec<String>,
    imports: Vec<String>,
    error_imports: Vec<String>,
}

#[derive(Serialize)]
struct ServerFileContext {
    file_stem: String,
    blocks: Vec<String>,
    error_imports: Vec<String>,
}

#[derive(Serialize)]
struct ErrorsFileContext {
    exceptions: Vec<ErrorsExceptionContext>,
}

#[derive(Serialize, Clone)]
struct ErrorsExceptionContext {
    ident: String,
    status: u16,
    headers: Vec<ErrorsExceptionMemberContext>,
    cookies: Vec<ErrorsExceptionMemberContext>,
    body: Vec<ErrorsExceptionFieldContext>,
}

#[derive(Serialize, Clone)]
struct ErrorsExceptionMemberContext {
    field: String,
    ty: TsType,
}

#[derive(Serialize, Clone)]
struct ErrorsExceptionFieldContext {
    field: String,
    ty: TsType,
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
    pub(crate) errors: String,
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
            .chain(&blocks.client)
            .chain(&blocks.server)
            .any(|block| ZodImportCollector::is_word_in_text(&schema_name, block));
        let has_mod_zod = blocks
            .zod
            .iter()
            .chain(&blocks.client)
            .chain(&blocks.server)
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
    // Document-level exceptions become typed error classes; the client and
    // server files import exactly the ones their operations reference.
    let exception_contexts: Vec<ErrorsExceptionContext> = rest_hir
        .document
        .exceptions
        .iter()
        .map(|exception| ErrorsExceptionContext {
            ident: exception.ident.clone(),
            status: exception.status,
            headers: exception
                .headers
                .iter()
                .map(|member| ErrorsExceptionMemberContext {
                    field: member.field.clone(),
                    ty: ts_type_for_type_spec(
                        &member.ty,
                        &exception.module_path,
                        TypeRefTarget::Client,
                    ),
                })
                .collect(),
            cookies: exception
                .cookies
                .iter()
                .map(|member| ErrorsExceptionMemberContext {
                    field: member.field.clone(),
                    ty: ts_type_for_type_spec(
                        &member.ty,
                        &exception.module_path,
                        TypeRefTarget::Client,
                    ),
                })
                .collect(),
            body: exception
                .body
                .iter()
                .map(|field| ErrorsExceptionFieldContext {
                    field: field.field.clone(),
                    ty: ts_type_for_type_spec(
                        &field.ty,
                        &exception.module_path,
                        TypeRefTarget::Client,
                    ),
                })
                .collect(),
        })
        .collect();
    let errors = if exception_contexts.is_empty() {
        String::new()
    } else {
        renderer.render_template(
            "http/errors.ts.j2",
            &ErrorsFileContext {
                exceptions: exception_contexts.clone(),
            },
        )?
    };
    let error_imports = |blocks: &[String]| -> Vec<String> {
        exception_contexts
            .iter()
            .filter(|exception| {
                blocks
                    .iter()
                    .any(|block| ZodImportCollector::is_word_in_text(&exception.ident, block))
            })
            .map(|exception| exception.ident.clone())
            .collect()
    };
    let client_error_imports = error_imports(&blocks.client);
    let server_error_imports = error_imports(&blocks.server);

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
                imports: zod_imports.clone(),
                blocks: blocks.zod,
            },
        )?,
        client: renderer.render_template(
            "http/client.ts.j2",
            &ClientFileContext {
                file_stem: file_stem.to_string(),
                blocks: blocks.client,
                imports: zod_imports.clone(),
                error_imports: client_error_imports,
            },
        )?,
        server: renderer.render_template(
            "http/server.ts.j2",
            &ServerFileContext {
                file_stem: file_stem.to_string(),
                blocks: blocks.server,
                error_imports: server_error_imports,
            },
        )?,
        errors,
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
        Self::collect_defs(&spec.0, &mut names);
        names.into_iter().collect()
    }

    fn collect_defs(defs: &[hir::Definition], names: &mut std::collections::BTreeSet<String>) {
        for def in defs {
            match def {
                hir::Definition::ModuleDcl(module) => {
                    Self::collect_defs(&module.definition, names);
                }
                hir::Definition::ConstrTypeDcl(constr) => {
                    Self::collect_constr_names(constr, names);
                }
                hir::Definition::TypeDcl(ty) => match ty {
                    hir::TypeDcl::ConstrTypeDcl(constr) => {
                        Self::collect_constr_names(constr, names);
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
