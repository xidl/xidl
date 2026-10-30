use serde::Serialize;
use xidl_parser::{
    hir,
    rest_hir::{HttpExceptionField, HttpExceptionMember, HttpOperation},
};

use crate::error::{IdlcError, IdlcResult};
use crate::generate::go_rest::{GoRestRenderContext, GoRestRenderer, MethodMeta};

#[derive(Serialize)]
/// One operation's projected exception encoding context.
pub(crate) struct ExceptionContext {
    name: String,
    status: u16,
    pub(super) has_body: bool,
    body: Vec<ExceptionField>,
    headers: Vec<ExceptionBinding>,
    cookies: Vec<ExceptionBinding>,
}

impl ExceptionContext {
    /// Resolves declared operation exceptions through the shared HTTP projection.
    pub(super) fn collect(
        operation: &HttpOperation,
        context: &GoRestRenderContext<'_>,
    ) -> IdlcResult<Vec<Self>> {
        operation
            .meta
            .raises
            .iter()
            .map(|reference| {
                let exception = context
                    .rest_hir
                    .document
                    .exceptions
                    .iter()
                    .find(|exception| {
                        exception.module_path == reference.module_path
                            && exception.ident == reference.ident
                    })
                    .ok_or_else(|| {
                        IdlcError::rpc(format!(
                            "operation '{}' exception '{}' is unresolved",
                            operation.meta.name, reference.ident
                        ))
                    })?;
                let mut name = reference.module_path.clone();
                name.push(reference.ident.clone());
                let scoped = hir::ScopedName {
                    is_root: false,
                    name,
                };
                let imported = context.packages.reference(&scoped);
                Ok(Self {
                    name: imported.unwrap_or_else(|| {
                        crate::generate::go::go_export_name(
                            &reference.module_path,
                            &reference.ident,
                        )
                    }),
                    status: exception.status,
                    has_body: !exception.body.is_empty(),
                    body: exception
                        .body
                        .iter()
                        .map(|field| ExceptionField::new(field, context))
                        .collect(),
                    headers: exception
                        .headers
                        .iter()
                        .map(ExceptionBinding::new)
                        .collect(),
                    cookies: exception
                        .cookies
                        .iter()
                        .map(ExceptionBinding::new)
                        .collect(),
                })
            })
            .collect()
    }
}

#[derive(Serialize)]
struct ExceptionField {
    field: String,
    wire: String,
    ty: String,
    optional: bool,
}

impl ExceptionField {
    fn new(field: &HttpExceptionField, context: &GoRestRenderContext<'_>) -> Self {
        let ty = crate::generate::go::go_type(&field.ty, &context.packages);
        Self {
            field: crate::generate::go::go_field_name(&field.field),
            wire: field.wire_name.clone(),
            ty: if field.optional { format!("*{ty}") } else { ty },
            optional: field.optional,
        }
    }
}

#[derive(Serialize)]
struct ExceptionBinding {
    field: String,
    wire: String,
    optional: bool,
    multi: bool,
}

impl ExceptionBinding {
    fn new(field: &HttpExceptionMember) -> Self {
        Self {
            field: crate::generate::go::go_field_name(&field.field),
            wire: field.wire_name.clone(),
            optional: field.optional,
            multi: field.is_multi,
        }
    }
}

#[derive(Serialize)]
/// One operation's exception codec, reused by HTTP and stream framing.
pub(super) struct ExceptionCodec<'a> {
    prefix: &'a str,
    response: String,
    exceptions: &'a [ExceptionContext],
}

impl<'a> ExceptionCodec<'a> {
    /// Borrows the operation's projected exception set.
    pub(super) fn new(method: &'a MethodMeta) -> Self {
        let response = if matches!(
            method.stream_kind,
            Some(xidl_parser::rest_hir::semantics::HttpStreamKind::Client)
        ) {
            method.response_struct.clone()
        } else {
            format!("*{}", method.response_struct)
        };
        Self {
            prefix: &method.struct_prefix,
            response,
            exceptions: &method.exceptions,
        }
    }

    /// Renders the shared status/body codec.
    pub(super) fn render(&self, out: &mut String, renderer: &GoRestRenderer) -> IdlcResult<()> {
        out.push_str(&renderer.render_template("exceptions.go.j2", self)?);
        out.push('\n');
        Ok(())
    }

    /// Renders a client-side HTTP error before decoding a success value.
    pub(super) fn render_http_error(
        &self,
        out: &mut String,
        renderer: &GoRestRenderer,
    ) -> IdlcResult<()> {
        out.push_str(&renderer.render_template("http_error.go.j2", self)?);
        out.push('\n');
        Ok(())
    }

    /// Renders a server-side HTTP error before any success output.
    pub(super) fn render_server_error(
        &self,
        out: &mut String,
        renderer: &GoRestRenderer,
    ) -> IdlcResult<()> {
        out.push_str(&renderer.render_template("server_error.go.j2", self)?);
        out.push('\n');
        Ok(())
    }
}
