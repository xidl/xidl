## Context

We need to implement Basic Auth support for axum in `xidl-rust-axum`, guided by
existing HTTP security docs and example usage. The current approach lacks a
reusable extractor and a consistent way to surface auth data in generated
handler inputs. The user wants Basic Auth to be exposed via an `xidl_auth` field
inside the request payload type `T` when authentication is enabled.

## Goals / Non-Goals

**Goals:**

- Provide a `BasicAuth` extractor type in `xidl-rust-axum/src/auth/basic.rs`
  that parses the Authorization header and supports optional password.
- When auth is enabled for a request, augment the generated request payload `T`
  with an `xidl_auth` field of type `BasicAuth`.
- Update docs and examples to show Basic Auth usage, realm configuration
  defaults (annotation or function name), and ensure
  `cargo b --example http_server` compiles.

**Non-Goals:**

- Implementing auth backends or user verification logic.
- Adding other auth schemes beyond Basic.
- Changing the overall axum routing or handler structure beyond the auth field
  addition.

## Decisions

- **Use an internal `BasicAuth` extractor in
  `xidl-rust-axum/src/auth/basic.rs`.** This localizes auth parsing and keeps
  the API stable even if external crates change.
  - _Alternatives considered:_ Directly depending on `axum-auth` types or using
    raw header parsing in handlers. Rejected due to tighter coupling or
    duplicated parsing logic.
- **Expose auth via `xidl_auth` inside `T` when auth is enabled.** This matches
  the requested shape and keeps handler signatures consistent with the request
  payload type.
  - _Alternatives considered:_ Wrapping inputs as `{ data: T, auth: BasicAuth }`
    or passing auth separately. Rejected per user request and to minimize
    handler signature divergence.
- **Treat password as optional in Basic Auth.** This aligns with Basic Auth
  specs where the password may be omitted in credentials.
- **Default realm when not configured.** Unauthorized responses must include
  `WWW-Authenticate: Basic realm="<realm>"`. If `@http_basic(realm=\"\")` is
  absent, use the handler function name as the realm.

## Risks / Trade-offs

- **[Risk] Handler input shape changes when auth is enabled** → Mitigation:
  scope change to auth-enabled endpoints only and update examples/docs
  accordingly.
- **[Risk] Parsing differences vs. `axum-auth` crate** → Mitigation: follow MDN
  Basic Auth header rules and include tests where practical.

## Migration Plan

- Add `BasicAuth` and parsing logic in `xidl-rust-axum`.
- Update code generation to inject `xidl_auth` into `T` when auth is enabled.
- Update `xidlc-examples/api/http/http_server.idl` usage and
  `docs/rfc/http-security.md`.
- Verify compilation with `cargo b --example http_server`.
