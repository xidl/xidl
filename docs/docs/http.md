# xidlc `http`

This page defines the IDL => HTTP behavior used by xidlc `rust-axum`
generation, including generated Axum code and `openapi.json`.

## Scope

- Covers interface-to-HTTP mapping.
- Covers route/method resolution, parameter placement, body encoding, and
  OpenAPI behavior.
- Type details follow the same core rules used by `rust` plus HTTP/OpenAPI
  specific mapping.

## Route and Method Resolution

For each interface operation:

- Default method: `POST`
- Default path: `/{module_path...}/{interface}/{method}`
- HTTP annotations override defaults:
  - `@get`, `@post`, `@put`, `@patch`, `@delete`, `@head`, `@options`
  - `path` in annotation overrides route path

## Parameter Source Rules

Per-parameter source is resolved in this order:

1. If parameter name appears in route template `{name}` => `Path`
2. Otherwise default by HTTP method:
   - `GET/DELETE/HEAD/OPTIONS` => `Query`
   - `POST/PUT/PATCH` => `Body`

## Request Body Encoding

Canonical JSON body rule:

- `0` body parameters: no request body
- `1` body parameter: direct value/schema
  - Example: payload is `{...}` (not `{ "req": {...} }`)
- `2+` body parameters: object keyed by parameter names
  - Example: `{ "a": ..., "b": ... }`

This is aligned across:

- Axum server extraction
- Axum client encoding
- OpenAPI requestBody schema

## Axum Generation Shape

Generated service trait method shape:

```rust
async fn method(&self, req: xidl_rust_axum::Request<RequestType>)
  -> Result<Ret, xidl_rust_axum::Error>;
```

Handler wiring:

- `Path` => `axum::extract::Path`
- `Query` => `axum::extract::Query`
- `Body` => `axum::Json`
- `HeaderMap` is always passed into `Request::new(headers, data)`

Client wiring:

- Path params replace placeholders in URL
- Query params serialized with `req.query(...)`
- Body serialized with `req.json(...)`

## Attribute Mapping

- `readonly attribute foo` => `GET .../foo`
- `attribute foo` => getter + setter:
  - `GET .../foo`
  - `POST .../set_foo`

Setter has one body parameter `value`, so request body is direct value.

## OpenAPI Mapping

Generated as OpenAPI `3.1.0`:

- `operationId`: `module.interface.method`
- `200` response: method return type (`void` => `null`)
- `500` response: shared `Error` schema
- Path/query/body placement follows the same source and body rules above

Info fields:

- Title: from `#progma xidlc package ...` (fallback `xidl`)
- Version: from `#progma xidlc openapi_version ...` (fallback `0.1.0`)

## Compatibility Note

Single-body-parameter requests are direct JSON values now. If a caller
previously sent wrapped payloads like `{ "req": {...} }`, it must be updated.
