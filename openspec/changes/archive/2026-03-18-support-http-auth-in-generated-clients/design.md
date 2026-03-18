## Context

Generated Rust HTTP/stream clients are produced from IDL, but they currently ignore security annotations. Servers enforce `@http_basic`, `@http_bearer`, and `@api_key`, which leads to 401 errors in integration tests and real usage unless callers manually inject headers. WebSocket-based bidi streams are especially impacted because the handshake currently cannot include auth headers.

## Goals / Non-Goals

**Goals:**
- Generated clients automatically attach required auth headers for endpoints that declare security.
- Support basic, bearer, and api key schemes defined by IDL for HTTP requests and WS handshakes.
- Provide a structured, ergonomic way to configure auth on generated clients.
- Preserve explicit opt-out with `@no_security` and allow per-call overrides when needed.

**Non-Goals:**
- Implement token refresh, OAuth flows, or credential storage.
- Add new auth schemes beyond those already defined in the IDL model.
- Change server-side auth behavior.

## Decisions

- **Add a client auth configuration object (per client instance).**
  - Rationale: Keeps configuration centralized and avoids per-method boilerplate. Allows default headers and scheme-specific settings (basic/bearer/api-key) and optional per-call override hooks.
  - Alternatives: Hardcode headers per method; require callers to set headers for every call. Rejected due to usability and high error risk.

- **Extend stream/WebSocket helpers to accept headers.**
  - Rationale: WS handshake is where auth must be applied for bidi streams. A header-aware connect function keeps the generator simple and avoids ad-hoc hacks.
  - Alternatives: Encode auth in query params. Rejected due to security and mismatch with IDL semantics.

- **Generate method-level auth application logic based on IDL security annotations.**
  - Rationale: IDL is the source of truth; clients should mirror server requirements. `@no_security` must suppress auth.
  - Alternatives: Global auth on all requests. Rejected because `@no_security` endpoints must remain unauthenticated.

## Risks / Trade-offs

- [Risk] Multiple security schemes on a single endpoint could be ambiguous → Mitigation: Follow existing server-side security resolution rules and document precedence.
- [Risk] Backwards compatibility for existing generated clients → Mitigation: Keep default behavior unchanged unless auth config is provided; provide a zero-config path that behaves like today.
- [Risk] Additional complexity in generated code → Mitigation: Centralize auth logic in a shared helper (e.g., `ClientAuth`) and keep generated methods thin.

## Migration Plan

- Add shared auth configuration and header injection utilities in `xidl-rust-axum`.
- Update codegen templates to emit auth-aware client structs and method calls.
- Update HTTP and stream tests to validate auth handling.
- Rollback: revert generator and helper changes; tests should continue to pass without auth handling.

## Open Questions

- Do we need per-request override hooks in the public client API, or is per-client auth config sufficient?
- How should multiple security requirements be prioritized if an endpoint declares more than one?
