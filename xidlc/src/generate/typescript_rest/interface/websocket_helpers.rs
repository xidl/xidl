use super::super::model::MethodModel;

pub(super) fn render_websocket_helpers(methods: &[MethodModel]) -> (Vec<String>, Vec<String>) {
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
