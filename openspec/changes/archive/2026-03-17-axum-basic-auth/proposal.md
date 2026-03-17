## Why

We need a reusable, documented Basic Auth extractor for axum to align with
existing HTTP security guidance and to make example servers compile with
authentication enabled. This change standardizes how Basic credentials are
parsed and exposed in generated handlers.

## What Changes

- Add a Basic Auth extractor type in `xidl-rust-axum` with optional password
  handling.
- When authentication is enabled for a request, extend the request payload type
  `T` with an `xidl_auth` field carrying `BasicAuth`.
- Update docs and examples to demonstrate Basic Auth usage and required headers.

## Capabilities

### New Capabilities

- `axum-basic-auth`: Provide a Basic Auth extractor and request payload
  augmentation for axum HTTP servers.

### Modified Capabilities

- (none)

## Impact

- `xidl-rust-axum` auth module additions and handler input struct shape changes
  when auth is enabled.
- `xidlc-examples` HTTP server example updates.
- Documentation updates in `docs/rfc/http-security.md` and related auth
  guidance.
