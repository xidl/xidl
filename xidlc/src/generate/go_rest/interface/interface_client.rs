use crate::error::IdlcResult;
use crate::generate::go_rest::{MethodMeta, definition};
use std::fmt::Write;
use xidl_parser::rest_hir::semantics::{HttpStreamCodec, HttpStreamKind};

use super::GoRestRenderer;
use super::interface_binding::{render_client_build_request, render_client_decode_response};

pub(super) fn render_client(
    out: &mut String,
    interface_name: &str,
    methods: &[MethodMeta],
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    writeln!(out, "type {interface_name}Client struct {{").unwrap();
    writeln!(out, "\tbaseURL string").unwrap();
    writeln!(out, "\thttpClient *http.Client").unwrap();
    writeln!(out, "\tauth xidlgohttp.ClientAuth").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "func New{interface_name}Client(baseURL string, httpClient *http.Client, auth xidlgohttp.ClientAuth) *{interface_name}Client {{"
    )
    .unwrap();
    writeln!(out, "\tif httpClient == nil {{").unwrap();
    writeln!(out, "\t\thttpClient = http.DefaultClient").unwrap();
    writeln!(out, "\t}}").unwrap();
    writeln!(
        out,
        "\treturn &{interface_name}Client{{baseURL: strings.TrimRight(baseURL, \"/\"), httpClient: httpClient, auth: auth}}"
    )
    .unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    for method in methods {
        match method.stream_kind {
            Some(HttpStreamKind::Server) => {
                render_client_server_stream(out, interface_name, method, renderer)?
            }
            Some(HttpStreamKind::Client) => {
                render_client_client_stream(out, interface_name, method)?
            }
            Some(HttpStreamKind::Bidi) => render_client_bidi_stream(out, interface_name, method)?,
            None => render_client_unary(out, interface_name, method, renderer)?,
        }
    }
    Ok(())
}

fn render_client_unary(
    out: &mut String,
    interface_name: &str,
    method: &MethodMeta,
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    writeln!(
        out,
        "func (c *{interface_name}Client) {}(ctx context.Context, req *{}) (*{}, error) {{",
        method.method_name, method.request_struct, method.response_struct
    )
    .unwrap();
    render_client_build_request(out, method, renderer)?;
    writeln!(out, "\tresp, err := c.httpClient.Do(httpReq)").unwrap();
    writeln!(out, "\tif err != nil {{ return nil, err }}").unwrap();
    writeln!(out, "\tdefer resp.Body.Close()").unwrap();
    writeln!(
        out,
        "\tif resp.StatusCode >= 400 {{ return nil, fmt.Errorf(\"http %d\", resp.StatusCode) }}"
    )
    .unwrap();
    render_client_decode_response(out, method);
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
    Ok(())
}

fn render_client_server_stream(
    out: &mut String,
    interface_name: &str,
    method: &MethodMeta,
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    let item_ty = method
        .return_ty
        .clone()
        .unwrap_or_else(|| "string".to_string());
    let reader_ty = if method.stream_codec == HttpStreamCodec::Sse {
        format!("*xidlgohttp.SSEStreamReader[{item_ty}]")
    } else {
        format!("*xidlgohttp.ServerStreamReader[{item_ty}]")
    };
    writeln!(
        out,
        "func (c *{interface_name}Client) {}(ctx context.Context, req *{}) ({}, error) {{",
        method.method_name, method.request_struct, reader_ty
    )
    .unwrap();
    render_client_build_request(out, method, renderer)?;
    writeln!(out, "\tresp, err := c.httpClient.Do(httpReq)").unwrap();
    writeln!(out, "\tif err != nil {{ return nil, err }}").unwrap();
    writeln!(out, "\tif resp.StatusCode >= 400 {{ defer resp.Body.Close(); return nil, fmt.Errorf(\"http %d\", resp.StatusCode) }}").unwrap();
    if method.stream_codec == HttpStreamCodec::Sse {
        writeln!(
            out,
            "\treturn xidlgohttp.NewSSEStreamReader[{item_ty}](resp.Body), nil"
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "\treturn xidlgohttp.NewNDJSONStreamReader[{item_ty}](resp.Body), nil"
        )
        .unwrap();
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
    Ok(())
}

fn render_client_client_stream(
    out: &mut String,
    interface_name: &str,
    method: &MethodMeta,
) -> IdlcResult<()> {
    writeln!(
        out,
        "func (c *{interface_name}Client) {}(ctx context.Context) (*xidlgohttp.ClientStreamWriter[{}, {}], error) {{",
        method.method_name, method.request_struct, method.response_struct
    )
    .unwrap();
    writeln!(
        out,
        "\trequestURL := c.baseURL + \"{}\"",
        method
            .paths
            .first()
            .cloned()
            .unwrap_or_else(|| "/".to_string())
    )
    .unwrap();
    writeln!(
        out,
        "\thttpReq, err := http.NewRequestWithContext(ctx, \"{}\", requestURL, nil)",
        definition::http_method_name(method.http_method)
    )
    .unwrap();
    writeln!(out, "\tif err != nil {{ return nil, err }}").unwrap();
    writeln!(
        out,
        "\thttpReq.Header.Set(\"Accept\", \"{}\")",
        method.response_content_type
    )
    .unwrap();
    writeln!(
        out,
        "\txidlgohttp.ApplyClientAuth(httpReq, c.auth, {}SecurityRequirements())",
        method.struct_prefix
    )
    .unwrap();
    writeln!(
        out,
        "\tstream := xidlgohttp.NewClientStreamWriter[{}, {}](ctx, c.httpClient, httpReq, func(resp *http.Response) ({}, error) {{",
        method.request_struct, method.response_struct, method.response_struct
    )
    .unwrap();
    writeln!(
        out,
        "\t\tif resp.StatusCode >= 400 {{ var zero {}; return zero, fmt.Errorf(\"http %d\", resp.StatusCode) }}",
        method.response_struct
    )
    .unwrap();
    writeln!(
        out,
        "\t\tdecoded, err := decode{}Response(resp)",
        method.struct_prefix
    )
    .unwrap();
    writeln!(out, "\t\treturn decoded, err").unwrap();
    writeln!(out, "\t}})").unwrap();
    writeln!(out, "\treturn stream, nil").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
    Ok(())
}

fn render_client_bidi_stream(
    out: &mut String,
    interface_name: &str,
    method: &MethodMeta,
) -> IdlcResult<()> {
    let mut params = vec!["ctx context.Context".to_string()];
    let mut req_fields = Vec::new();
    for param in &method.handshake_params {
        let ty = if param.optional {
            format!("*{}", param.ty)
        } else {
            param.ty.clone()
        };
        let name = param.field_name.to_ascii_lowercase();
        params.push(format!("{name} {ty}"));
        req_fields.push(format!("{}: {name},", param.field_name));
    }
    writeln!(
        out,
        "func (c *{interface_name}Client) {}({}) (*xidlgohttp.WSBidiStream[{}, {}], error) {{",
        method.method_name,
        params.join(", "),
        method.stream_out_ty,
        method.stream_in_ty
    )
    .unwrap();
    writeln!(
        out,
        "\treq := &{}{{{}}}",
        method.request_struct,
        req_fields.join(" ")
    )
    .unwrap();
    writeln!(out, "\tpath := format{}Path(req)", method.struct_prefix).unwrap();
    writeln!(out, "\tbase := strings.TrimRight(c.baseURL, \"/\")").unwrap();
    writeln!(
        out,
        "\tbase = strings.Replace(base, \"http://\", \"ws://\", 1)"
    )
    .unwrap();
    writeln!(
        out,
        "\tbase = strings.Replace(base, \"https://\", \"wss://\", 1)"
    )
    .unwrap();
    writeln!(out, "\tendpoint := base + path").unwrap();
    let subproto = method
        .websocket_subprotocol
        .as_deref()
        .map(|value| format!("{value:?}"))
        .unwrap_or_else(|| "\"\"".to_string());
    writeln!(out, "\tvar subprotocols []string").unwrap();
    writeln!(
        out,
        "\tif {subproto} != \"\" {{ subprotocols = []string{{{subproto}}} }}"
    )
    .unwrap();
    writeln!(
        out,
        "\tconn, err := xidlgohttp.DialWebSocket(ctx, endpoint, subprotocols)"
    )
    .unwrap();
    writeln!(out, "\tif err != nil {{ return nil, err }}").unwrap();
    writeln!(
        out,
        "\treturn xidlgohttp.NewWSBidiClient[{}, {}](conn), nil",
        method.stream_in_ty, method.stream_out_ty
    )
    .unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
    Ok(())
}
