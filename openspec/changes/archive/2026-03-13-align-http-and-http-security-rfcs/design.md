## Context

The unary HTTP and HTTP security RFCs were tightened recently, but the current
implementation still reflects older behavior. The main mismatches are:

- request decoding currently does not consistently implement "missing means
  zero/default unless `@optional`"
- deprecation metadata is documented but not yet modeled end-to-end
- HTTP success/error defaults are only partially reflected in generated OpenAPI
  and runtime behavior
- HTTP security annotations proposed in the RFCs are not yet wired through the
  parser, HIR, and generators

This is a cross-cutting change touching parser/HIR, HTTP code generation,
OpenAPI generation, and the axum runtime. That makes an explicit design useful
before implementation starts.

## Goals / Non-Goals

**Goals:**
- Align unary HTTP runtime and generated artifacts with the new unary HTTP RFC
- Add parser/HIR support for unary HTTP security annotations
- Propagate unary HTTP and HTTP security metadata into generated OpenAPI
- Add validation and tests for new annotation forms and new default-value rules

**Non-Goals:**
- Implement HTTP stream behavior
- Implement security enforcement middleware or a standardized auth context API
- Add a standard annotation for custom success status codes
- Fully model every OpenAPI security feature

## Decisions

### 1. Treat RFC alignment as a pipeline change, not a generator-only change

Decision:
- Update the annotation pipeline from parser to HIR first, then adapt the HTTP,
  OpenAPI, and axum layers.

Rationale:
- The new behavior depends on annotations such as `@deprecated`, `@http-basic`,
  `@http-bearer`, `@api-key`, `@oauth2`, and `@no-security`.
- If the compiler pipeline cannot represent them cleanly, later layers will
  either duplicate parsing logic or drift in behavior.

Alternatives considered:
- Patch only the generators.
  Rejected because it bakes annotation semantics into multiple code paths and
  makes validation inconsistent.

### 2. Keep missing-value semantics target-aware, but validate unsupported cases early

Decision:
- Implement the RFC default-value table for types with stable defaults and fail
  compilation or request decoding for omitted values whose target mapping cannot
  produce a stable default.

Rationale:
- The RFC intentionally leaves room for target-language constraints.
- Rust can materialize defaults for many primitives and containers but not for
  every constructed type without extra policy.

Alternatives considered:
- Force every omitted non-optional value to decode somehow.
  Rejected because it would invent values for types that have no stable
  semantics.
- Require `@optional` everywhere.
  Rejected because it contradicts the RFC direction and makes generated APIs
  noisy.

### 3. Use generated OpenAPI as the canonical documentation projection

Decision:
- Reflect RFC behavior into OpenAPI generation rather than inventing a second
  documentation layer.

Rationale:
- The repository already uses generated OpenAPI artifacts as the externally
  consumable contract for HTTP APIs.
- This keeps deprecation and security metadata visible without requiring runtime
  headers or dynamic behavior to be present first.

Alternatives considered:
- Implement runtime metadata only and leave OpenAPI behind.
  Rejected because it would keep contract drift visible to users.

### 4. Defer runtime auth enforcement and standard auth context design

Decision:
- Implement parsing, validation, and OpenAPI emission for security annotations,
  but do not standardize enforcement hooks or authenticated principal plumbing
  in this change.

Rationale:
- The RFC explicitly keeps enforcement implementation-specific.
- Standardizing runtime auth context would broaden scope into framework API
  design and likely stall this alignment work.

Alternatives considered:
- Add a first-class auth context to `xidl-rust-axum::Request`.
  Rejected as premature and not required to make the RFC contract real.

## Risks / Trade-offs

- [Risk] Missing-value semantics may differ across existing generated tests and
  examples. -> Mitigation: update snapshots deliberately and add focused tests
  for zero/default versus `@optional`.
- [Risk] Hyphenated annotation names may expose parser assumptions. ->
  Mitigation: start with parser/HIR tests before touching generators.
- [Risk] OpenAPI security projection may need scheme naming conventions that the
  RFC intentionally leaves implicit. -> Mitigation: define deterministic
  generator-side scheme IDs in the implementation without changing RFC text.
- [Risk] Deprecation metadata may be represented in OpenAPI but not emitted at
  runtime yet. -> Mitigation: document runtime header emission as optional and
  keep this change focused on static contract alignment.

## Migration Plan

1. Extend parser/HIR support and validation for the new HTTP and HTTP security
   annotations.
2. Update unary HTTP generation and runtime decoding to match the RFC default
   value and media-type rules.
3. Update OpenAPI generation to emit deprecation and security metadata.
4. Refresh snapshots and examples, then run focused tests for parser, codegen,
   and axum runtime behavior.

Rollback strategy:
- Revert the change as a single compiler/runtime update if generated code or
  snapshots reveal unacceptable compatibility fallout.
- Because this work changes generated outputs, rollback is primarily a source
  revert rather than a runtime-only toggle.

## Open Questions

- For generated OpenAPI security schemes, what deterministic scheme names should
  be used for `@http-basic`, `@http-bearer`, `@api-key`, and `@oauth2`?
- Should `@deprecated` metadata be emitted into runtime response headers in the
  same change, or remain documentation-only for the first implementation pass?
- Which existing example IDLs should be expanded to exercise `@api-key` and
  `@oauth2`, given that runtime enforcement remains out of scope?
