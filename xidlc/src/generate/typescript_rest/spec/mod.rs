use crate::error::IdlcResult;
use crate::generate::typescript::TypescriptRenderer;
use crate::generate::typescript::definition::TypeRefTarget;
use crate::generate::typescript::definition::annotations::ts_doc_lines;
use crate::generate::typescript::definition::contexts::TsType;
use crate::generate::typescript::definition::type_expr::ts_type_for_type_spec;
use serde::Serialize;
use xidl_parser::hir;
use xidl_parser::rest_hir::RestHirDocument;

use super::interface::render_interface;
use super::model::{TsHttpBlocks, TsRaisesContext};
use zod_imports::ZodImportCollector;

#[cfg(test)]
mod tests;
mod zod_imports;

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
    has_errors: bool,
}

#[derive(Serialize)]
struct ServerFileContext {
    file_stem: String,
    blocks: Vec<String>,
    has_errors: bool,
}

#[derive(Serialize)]
struct ErrorsFileContext {
    file_stem: String,
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
    optional: bool,
    ty: TsType,
}

#[derive(Serialize, Clone)]
struct ErrorsExceptionFieldContext {
    field: String,
    optional: bool,
    ty: TsType,
}

#[derive(Serialize)]
struct ModuleContext {
    ident: String,
    doc: Vec<String>,
    blocks: Vec<String>,
}

pub(crate) struct TsHttpOutput {
    pub(crate) types: String,
    pub(crate) zod: String,
    pub(crate) client: String,
    pub(crate) server: String,
    pub(crate) errors: String,
}

/// Re-prints every rendered TypeScript file with the canonical formatter.
///
/// The templates emit the code's shape; the formatter owns the layout. A file
/// that does not parse is left untouched rather than failing the whole
/// generation, so a template regression surfaces as an unformatted artifact
/// instead of a hard error.
pub(crate) fn format_output(mut output: TsHttpOutput) -> IdlcResult<TsHttpOutput> {
    output.types = format_source(&output.types);
    output.zod = format_source(&output.zod);
    output.client = format_source(&output.client);
    output.server = format_source(&output.server);
    output.errors = format_source(&output.errors);
    Ok(output)
}

fn format_source(source: &str) -> String {
    if source.trim().is_empty() {
        return source.to_string();
    }
    match crate::fmt::format_typescript_source(source) {
        Ok(formatted) => formatted,
        Err(_) => source.to_string(),
    }
}

pub(crate) fn render_spec(
    spec: &hir::Specification,
    file_stem: &str,
    renderer: &TypescriptRenderer,
    rest_hir: &RestHirDocument,
) -> IdlcResult<TsHttpOutput> {
    let text_aliases = super::interface::text_aliases::TextAliases::collect(spec);
    let blocks = render_defs(&spec.0, &[], renderer, rest_hir, &text_aliases)?;
    let mut model_exports = ZodImportCollector::collect_exported_names(spec);
    model_exports.extend(renderer.imported_units());
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
            ident: TsRaisesContext::error_ident(exception, &rest_hir.document.exceptions),
            status: exception.status,
            headers: exception
                .headers
                .iter()
                .map(|member| ErrorsExceptionMemberContext {
                    field: member.field.clone(),
                    optional: member.optional,
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
                    optional: member.optional,
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
                    optional: field.optional,
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
                file_stem: file_stem.to_string(),
                exceptions: exception_contexts.clone(),
            },
        )?
    };
    let has_errors = |blocks: &[String]| blocks.iter().any(|block| block.contains("$xidlErrors."));
    let client_has_errors = has_errors(&blocks.client);
    let server_has_errors = has_errors(&blocks.server);

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
                has_errors: client_has_errors,
            },
        )?,
        server: renderer.render_template(
            "http/server.ts.j2",
            &ServerFileContext {
                file_stem: file_stem.to_string(),
                blocks: blocks.server,
                has_errors: server_has_errors,
            },
        )?,
        errors,
    })
}

#[allow(clippy::too_many_arguments)]
fn render_defs(
    defs: &[hir::Definition],
    module_path: &[String],
    renderer: &TypescriptRenderer,
    rest_hir: &RestHirDocument,
    text_aliases: &super::interface::text_aliases::TextAliases,
) -> IdlcResult<TsHttpBlocks> {
    let mut out = TsHttpBlocks::default();
    for def in defs {
        match def {
            hir::Definition::ModuleDcl(module) => {
                let mut next = module_path.to_vec();
                next.push(module.ident.clone());
                let body =
                    render_defs(&module.definition, &next, renderer, rest_hir, text_aliases)?;
                if !body.is_empty() {
                    let ident =
                        crate::generate::typescript::definition::names::ts_ident(&module.ident);
                    let doc = ts_doc_lines(&module.annotations)?;
                    let module = ModuleBlock {
                        renderer,
                        ident: &ident,
                        doc: &doc,
                    };
                    out.types.push(module.render(&body.types.join("\n"))?);
                    out.zod.push(module.render(&body.zod.join("\n"))?);
                    out.client.push(module.render(&body.client.join("\n"))?);
                    out.server.push(module.render(&body.server.join("\n"))?);
                }
            }
            hir::Definition::InterfaceDcl(interface) => {
                out.extend(render_interface(
                    interface,
                    module_path,
                    renderer,
                    rest_hir,
                    text_aliases,
                )?);
            }
            _ => {}
        }
    }
    Ok(out)
}

/// One namespace block of the HTTP output, rendered once per file kind.
struct ModuleBlock<'a> {
    renderer: &'a TypescriptRenderer,
    ident: &'a str,
    doc: &'a [String],
}

impl ModuleBlock<'_> {
    fn render(&self, body: &str) -> IdlcResult<String> {
        self.renderer.render_template(
            "http/module.ts.j2",
            &ModuleContext {
                ident: self.ident.to_string(),
                doc: self.doc.to_vec(),
                blocks: vec![crate::generate::typescript::definition::names::indent_block(body, 1)],
            },
        )
    }
}
