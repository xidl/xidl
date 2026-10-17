use super::super::model::MethodModel;
use crate::error::IdlcResult;
use crate::generate::typescript::TypescriptRenderer;
use crate::generate::typescript::definition::contexts::{FieldTypeContext, TsType};
use crate::generate::typescript::definition::names::ts_prop_name;
use serde::Serialize;
use xidl_parser::hir::VersionParameter;

#[derive(Serialize)]
pub(super) struct WebsocketHelpers<'a> {
    name: String,
    path: &'a str,
    subprotocol: Option<&'a str>,
    stream_in: TsType,
    stream_out: TsType,
    version_parameter: Option<&'a VersionParameter>,
    api_version: Option<&'a str>,
}

impl WebsocketHelpers<'_> {
    fn render_legacy(methods: &[MethodModel]) -> (Vec<String>, Vec<String>) {
        let mut client = Vec::new();
        let mut server = Vec::new();
        for method in methods {
            if !method.is_websocket {
                continue;
            }
            let subproto = method
                .websocket_subprotocol
                .as_deref()
                .map(|value| format!("{value:?}"))
                .unwrap_or_else(|| "undefined".to_string());
            let name = method.name.to_ascii_lowercase();
            let stream_in = &method.stream_in_ty;
            let stream_out = &method.stream_out_ty;
            let path = &method.path;

            client.push(format!(
                r#"
export async function open_{name}Session(
  baseUrl: string,
  options?: {{ subprotocol?: string; WebSocketImpl?: typeof WebSocket }},
): Promise<import('xidl-typescript-client').WsBidiSession<{stream_in}, {stream_out}>> {{
  const {{ openWsBidiClient }} = await import('xidl-typescript-client');
  const path = {path:?};
  const url = baseUrl.replace(/\/$/, '') + path;
  return openWsBidiClient<{stream_in}, {stream_out}>(url, {{
    subprotocol: {subproto},
    ...options,
  }});
}}
"#
            ));

            server.push(format!(
                r#"
export function create_{name}Handler(
  impl: {{
    {name}(session: import('xidl-typescript-server').WsBidiServerSession<{stream_in}, {stream_out}>): Promise<void>;
  }},
  runtime: {{
    wrapWsBidiServer<TIn, TOut>(
      socket: import('xidl-typescript-server').WsLikeSocket,
    ): import('xidl-typescript-server').WsBidiServerSession<TIn, TOut>;
  }},
) {{
  return (socket: import('xidl-typescript-server').WsLikeSocket) => {{
    const session = runtime.wrapWsBidiServer<{stream_in}, {stream_out}>(socket);
    return impl.{name}(session).finally(() => session.close());
  }};
}}
export const {name}_ws_subprotocol: string | undefined = {subproto};
"#
            ));
        }
        (client, server)
    }

    fn input_type(method: &MethodModel) -> TsType {
        let handshake = method
            .path_params
            .iter()
            .map(|param| param.key_name.as_str())
            .chain(
                method
                    .query_params
                    .iter()
                    .map(|param| param.key_name.as_str()),
            )
            .chain(
                method
                    .header_params
                    .iter()
                    .map(|param| param.key_name.as_str()),
            )
            .chain(
                method
                    .cookie_params
                    .iter()
                    .map(|param| param.key_name.as_str()),
            )
            .map(ts_prop_name)
            .collect::<std::collections::HashSet<_>>();
        let fields = method
            .request_fields
            .iter()
            .filter(|field| !handshake.contains(&field.prop))
            .map(|field| FieldTypeContext {
                prop: field.prop.clone(),
                ty: field.ty.clone(),
                optional: field.optional,
                doc: Vec::new(),
            })
            .collect::<Vec<_>>();
        if fields.is_empty() {
            TsType::Primitive("never".into())
        } else {
            TsType::InlineStruct(fields)
        }
    }

    pub(super) fn render(
        methods: &[MethodModel],
        renderer: &TypescriptRenderer,
        version: Option<(&VersionParameter, &str)>,
    ) -> IdlcResult<(Vec<String>, Vec<String>)> {
        if version.is_none() {
            return Ok(Self::render_legacy(methods));
        }
        let mut client = Vec::new();
        let mut server = Vec::new();
        for method in methods.iter().filter(|method| method.is_websocket) {
            let context = WebsocketHelpers {
                name: method.name.to_ascii_lowercase(),
                path: &method.path,
                subprotocol: method.websocket_subprotocol.as_deref(),
                stream_in: Self::input_type(method),
                stream_out: if method.response_fields.is_empty() {
                    TsType::Primitive("never".into())
                } else {
                    method.return_ty.clone()
                },
                version_parameter: version.map(|(parameter, _)| parameter),
                api_version: version.map(|(_, value)| value),
            };
            client.push(renderer.render_template("http/websocket_client.ts.j2", &context)?);
            server.push(renderer.render_template("http/websocket_server.ts.j2", &context)?);
        }
        Ok((client, server))
    }
}
