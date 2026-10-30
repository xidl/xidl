use serde::Serialize;

use super::interface_binding::render_client_build_request;
use crate::error::IdlcResult;
use crate::generate::go_rest::{GoRestRenderer, MethodMeta};

#[derive(Serialize)]
/// Rendering inputs shared by a stream operation's client and server.
pub(super) struct StreamMethod<'a> {
    prefix: &'a str,
    name: &'a str,
    request: &'a str,
    response: &'a str,
    item: &'a str,
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
            meta: method,
        }
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
