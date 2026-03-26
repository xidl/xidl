# Annotations Reference

This page lists the major annotations used by the current XIDL toolchain.

## Core and shaping annotations

### `@optional`

Purpose:

- preserve omission semantics for fields or parameters

Important notes:

- used by HTTP/OpenAPI handling, Rust data mapping, and TypeScript generation
- invalid for HTTP path parameters

See also:

- [IDL Guide](../user/idl.md)
- [HTTP Guide](../user/http.md)
- [HTTP RFC](../rfc/http.md)

### `@name("...")`

Purpose:

- rename a field for serialization or wire/property naming

Current note:

- documented previously as an XIDL extension and commonly aligned with
  serialization naming needs

## HTTP annotations

Supported HTTP verb annotations:

- `@get`
- `@post`
- `@put`
- `@patch`
- `@delete`
- `@head`
- `@options`

Routing and parameter-source annotations:

- `@path`
- `@query`
- `@header`
- `@cookie`

Content and lifecycle annotations:

- `@Consumes`
- `@Produces`
- `@deprecated`

Use these with the [HTTP Guide](../user/http.md) and [HTTP RFC](../rfc/http.md).

## Stream annotations

Current stream annotations used across HTTP and JSON-RPC profiles:

- `@server_stream`
- `@client_stream`
- `@bidi_stream`
- `@stream_codec(...)`

Practical note:

- stream support exists in the repository but is more implementation-led than
  the mature unary mappings

## HTTP security annotations

Current security annotations:

- `@no_security`
- `@http_basic`
- `@http_bearer`
- `@api_key(...)`
- `@oauth2(...)`

Use these with:

- [HTTP Guide](../user/http.md)
- [HTTP Security RFC](../rfc/http-security.md)

## Rust passthrough annotations

This repository supports Rust-oriented passthrough attributes such as:

- `@rust-serde(...)`
- other `@rust-...` forms emitted as raw Rust attributes in generated output

Practical note:

- passthrough annotations affect Rust-emitting generators only
- source order is preserved where supported

## Other target-specific behavior

Some annotations only make sense in a subset of generators. When in doubt:

1. check the user guide for the transport or target
2. check the target mapping docs under `xidlc/doc/`
3. inspect the relevant generator under `xidlc/src/generate/`
