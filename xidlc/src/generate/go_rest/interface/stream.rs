use serde::Serialize;
use xidl_parser::{hir, rest_hir::HttpOperation};

use super::interface_binding::render_client_build_request;
use crate::error::{IdlcError, IdlcResult};
use crate::generate::go_rest::{GoRestRenderContext, GoRestRenderer, MethodMeta, definition};

#[derive(Serialize)]
/// One operation's projected exception encoding context.
pub(crate) struct StreamExceptionContext {
    name: String,
    status: u16,
    pub(super) has_body: bool,
    required: Vec<String>,
}

impl StreamExceptionContext {
    /// Resolves declared stream exceptions through the shared HTTP projection.
    pub(super) fn collect(
        operation: &HttpOperation,
        context: &GoRestRenderContext<'_>,
    ) -> IdlcResult<Vec<Self>> {
        if operation.meta.stream.kind.is_none() {
            return Ok(Vec::new());
        }
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
                            "stream '{}' exception '{}' is unresolved",
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
                let canonical = hir::TypeSpec::ScopedName(hir::ScopedName {
                    is_root: true,
                    ..scoped
                });
                Ok(Self {
                    name: imported.unwrap_or_else(|| {
                        definition::go_type(
                            &canonical,
                            &context.packages,
                            &[],
                            context.versioned_modules(),
                        )
                    }),
                    status: exception.status,
                    has_body: !exception.body.is_empty(),
                    required: exception
                        .body
                        .iter()
                        .filter(|field| !field.optional)
                        .map(|field| field.wire_name.clone())
                        .collect(),
                })
            })
            .collect()
    }
}

#[derive(Serialize)]
/// Rendering inputs shared by a stream operation's client and server.
pub(super) struct StreamMethod<'a> {
    prefix: &'a str,
    name: &'a str,
    request: &'a str,
    response: &'a str,
    item: &'a str,
    exceptions: &'a [StreamExceptionContext],
    #[serde(skip)]
    meta: &'a MethodMeta,
}

impl<'a> StreamMethod<'a> {
    /// Borrows the normalized operation metadata.
    pub(super) fn new(method: &'a MethodMeta) -> Self {
        Self {
            prefix: &method.struct_prefix,
            name: &method.method_name,
            request: &method.request_struct,
            response: &method.response_struct,
            item: method.return_ty.as_deref().unwrap_or("string"),
            exceptions: &method.stream_exceptions,
            meta: method,
        }
    }

    /// Renders the shared HTTP and terminal-frame exception codec.
    pub(super) fn render_errors(
        &self,
        out: &mut String,
        renderer: &GoRestRenderer,
    ) -> IdlcResult<()> {
        out.push_str(&renderer.render_template("stream_errors.go.j2", self)?);
        out.push('\n');
        Ok(())
    }

    /// Renders a server-stream handler.
    pub(super) fn render_server(
        &self,
        out: &mut String,
        renderer: &GoRestRenderer,
    ) -> IdlcResult<()> {
        out.push_str(&renderer.render_template("stream_server.go.j2", self)?);
        out.push('\n');
        Ok(())
    }

    /// Renders a client-stream handler.
    pub(super) fn render_upload(
        &self,
        out: &mut String,
        renderer: &GoRestRenderer,
    ) -> IdlcResult<()> {
        out.push_str(&renderer.render_template("stream_upload.go.j2", self)?);
        out.push('\n');
        Ok(())
    }

    /// Renders the pre-output HTTP exception response.
    pub(super) fn render_http_error(
        &self,
        out: &mut String,
        renderer: &GoRestRenderer,
    ) -> IdlcResult<()> {
        out.push_str(&renderer.render_template("stream_http_error.go.j2", self)?);
        out.push('\n');
        Ok(())
    }

    /// Renders the client request around the existing request-binding renderer.
    pub(super) fn render_client(
        &self,
        out: &mut String,
        interface: &str,
        renderer: &GoRestRenderer,
    ) -> IdlcResult<()> {
        out.push_str(&renderer.render_template(
            "stream_client_start.go.j2",
            &minijinja::context!(interface, method => self),
        )?);
        out.push('\n');
        render_client_build_request(out, self.meta, renderer)?;
        out.push_str(&renderer.render_template("stream_client_finish.go.j2", self)?);
        out.push('\n');
        Ok(())
    }
}
