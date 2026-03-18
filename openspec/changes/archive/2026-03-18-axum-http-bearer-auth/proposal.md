## Why

The axum HTTP server example and security docs currently focus on basic auth; we
need a first-class Bearer token flow to match common API usage and align with
existing RFC guidance. This enables consistent extraction, typing, and codegen
for Bearer auth across generated servers.

## What Changes

- Add Bearer authentication support for axum HTTP servers, including typed
  extractor and request shaping.
- Introduce shared auth structs (user/password extractor and bearer auth) in
  `xidl-rust-axum` for reuse.
- Update HTTP server mapping/codegen to add `xidl_auth` to request types when
  required (e.g.,
  `xidl_rust_axum::Request<{ ...data, xidl_auth: BearerAuth }>`).
- Ensure empty `Authorization: Bearer` yields default string token value.
- Update docs and examples to demonstrate Bearer auth usage.

## Capabilities

### New Capabilities

- `axum-bearer-auth`: Bearer token authentication support in axum HTTP server
  codegen and runtime.

### Modified Capabilities

- `http-security-mapping`: Add/adjust requirements to map Bearer auth into
  generated request types and extractors.

## Impact

- Code: `xidl-rust-axum/src/auth/bearer.rs`, axum HTTP server codegen, example
  server mapping in `xidlc-examples/api/http/http_server.idl`.
- Docs: `docs/rfc/http-security.md` and related HTTP security guidance.
- Public API: Generated request types for authenticated endpoints gain
  `xidl_auth` field on `xidl_rust_axum::Request<T>`.
