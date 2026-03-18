## 1. Auth Foundations

- [x] 1.1 Add client auth configuration types (basic/bearer/api-key) in `xidl-rust-axum`
- [x] 1.2 Implement header injection helpers for HTTP requests based on auth config
- [x] 1.3 Add websocket connect helper that accepts headers for auth-required streams

## 2. Codegen Updates

- [x] 2.1 Extend HTTP client codegen to attach auth headers per endpoint security
- [x] 2.2 Extend stream client codegen to attach auth headers for SSE/NDJSON endpoints
- [x] 2.3 Extend bidi stream client codegen to pass auth headers during WS handshake
- [x] 2.4 Add per-call auth override hooks in generated clients

## 3. Tests and Examples

- [x] 3.1 Update example tests to use client auth configuration (http + stream)
- [x] 3.2 Add/extend security coverage tests for client auth behavior
- [x] 3.3 Validate `@no_security` endpoints do not receive auth headers

## 4. Documentation

- [x] 4.1 Document client auth configuration in relevant README/docs
