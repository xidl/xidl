## Why

The repository already has an HTTP stream RFC draft and partial experimental
support, but the generated Axum, OpenAPI, and TypeScript outputs do not yet form
one clear, RFC-aligned contract. This change closes that gap so stream-enabled
IDLs can be compiled into consistent server/runtime, API description, and client
artifacts.

## What Changes

- Define the supported HTTP stream behavior for Rust Axum generation and the
  `xidl-rust-axum` runtime, including server-stream and client-stream request /
  response handling.
- Upgrade generated OpenAPI output from 3.1.0 to 3.2.0 and define how HTTP
  stream operations are projected there, including stream media types,
  `itemSchema`, and supported method constraints.
- Define how HTTP stream operations are projected into generated TypeScript
  clients, including SSE readers, NDJSON request streams, and target-side
  limitations.
- Add example and verification coverage so the generated outputs for the three
  targets stay aligned with the HTTP stream RFC draft.

## Capabilities

### New Capabilities
- `http-stream-axum`: Defines RFC-aligned Rust Axum code generation and runtime
  behavior for HTTP server-stream and client-stream operations.
- `http-stream-openapi`: Defines how HTTP stream operations appear in generated
  OpenAPI documents, including media types and schema projection.
- `http-stream-typescript`: Defines how HTTP stream operations appear in
  generated TypeScript clients, including supported streaming patterns and
  client helper behavior.

### Modified Capabilities
- None.

## Impact

- Affected code: `xidlc` stream metadata and generators, `xidl-rust-axum`
  streaming runtime/helpers, TypeScript client templates/helpers, OpenAPI
  emission, and example build outputs.
- Affected APIs: generated Rust Axum server/client interfaces, generated
  OpenAPI 3.2.0 stream descriptions, and generated TypeScript client method
  signatures/stream helpers.
- Risk areas: keeping stream media types and method restrictions consistent
  across all three targets, preserving existing experimental examples, and
  clarifying which HTTP stream RFC features remain out of scope for now.

## Change Notes

- Generated OpenAPI documents now emit `openapi: 3.2.0`.
- Generated OpenAPI stream media types now use `itemSchema` instead of plain
  `schema` for SSE and NDJSON stream bodies.
- Generated OpenAPI JSON field ordering changed in snapshots because the final
  document is patched as JSON after `utoipa` serialization.
- TypeScript generation keeps rejecting unsupported bidi and non-body
  client-stream shapes with explicit validation errors.
