## Context

We generate axum HTTP servers from XIDL, with existing security guidance focused
on basic auth. We need a Bearer token flow that matches axum header extraction
patterns and maps auth info into generated request types. The user expects
shared auth structs to live in `xidl-rust-axum/src/auth/bearer.rs`, and
authenticated requests to be represented as
`xidl_rust_axum::Request<{ ...data, xidl_auth: BearerAuth }>` (i.e., add
`xidl_auth` onto `T`).

## Goals / Non-Goals

**Goals:**

- Provide a first-class Bearer auth extractor and runtime type in
  `xidl-rust-axum`.
- Update HTTP server codegen/mapping so authenticated endpoints receive
  `xidl_rust_axum::Request<T>` where `T` includes `xidl_auth`.
- Define how empty `Authorization: Bearer` is handled (default string token).
- Align docs and example IDL with the new Bearer flow.

**Non-Goals:**

- Implement OAuth token validation or external auth providers.
- Change unrelated HTTP security behaviors beyond mapping for Bearer auth.
- Replace axum’s header extraction approach.

## Decisions

- **Use `TypedHeader<BearerHeader>` (from `axum_extra`) for Bearer parsing.**
  - Rationale: axum 0.8 moves typed headers into `axum_extra`; a custom
    `BearerHeader` allows empty tokens while still using typed extraction.
  - Alternatives: `Authorization<Bearer>` or `axum-auth`. Rejected to support
    the empty-token requirement and avoid extra dependencies.

- **Centralize auth structs in `xidl-rust-axum/src/auth/bearer.rs`.**
  - Rationale: user requirement and keeps auth-related extractors in one place.
  - Alternatives: split by scheme (basic vs bearer) into separate files.
    Rejected to meet the request for shared/common structs.

- **Request shaping for authenticated endpoints adds `xidl_auth` onto `T`, used
  as `xidl_rust_axum::Request<T>`.**
  - Rationale: explicit auth field while preserving the existing request wrapper
    type.
  - Alternatives: separate `{ data, xidl_auth }` wrapper. Rejected per clarified
    requirement.

- **Empty `Authorization: Bearer` yields default token value.**
  - Rationale: explicit behavior requested; avoid treating empty token as an
    error.
  - Alternatives: reject empty token with 401. Rejected per requirement.

## Risks / Trade-offs

- **[Risk]** Generated request type changes may be breaking for existing
  endpoints that add Bearer auth. **→ Mitigation:** Scope changes only to
  endpoints with Bearer auth; document and update examples.
- **[Risk]** Ambiguity if both basic and bearer auth are supported in the same
  surface. **→ Mitigation:** Keep scheme-specific types and mapping explicit in
  spec; avoid implicit defaults.
