## Why

Generated HTTP clients currently ignore `@http_basic`, `@http_bearer`, and `@api_key` security declarations, causing 401s in real usage and forcing ad-hoc test workarounds. We need first-class client auth support to make generated clients usable and aligned with the IDL security contract.

## What Changes

- Generate client-side auth handling for HTTP and streaming APIs based on IDL security annotations (`@http_basic`, `@http_bearer`, `@api_key`, `@no_security`).
- Add a structured way to configure auth on generated clients (default headers / per-call overrides).
- Ensure websocket/bidi stream handshakes include auth headers when required.
- Extend tests to validate client auth behavior for unary and stream endpoints.

## Capabilities

### New Capabilities
- `generated-client-auth`: Generated clients automatically apply required HTTP auth schemes (basic, bearer, api key) for HTTP and stream endpoints, including WS handshakes.

### Modified Capabilities
- (none)

## Impact

- Codegen outputs for Rust HTTP/stream clients.
- xidl-rust-axum client utilities and stream connection helpers.
- Example tests and security coverage tests for HTTP and stream APIs.
