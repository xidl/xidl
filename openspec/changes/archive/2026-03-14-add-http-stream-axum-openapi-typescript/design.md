## Context

The repository already contains three relevant pieces of stream support:

- the HTTP stream RFC draft in `docs/rfc/http-stream.md`
- partial generator/runtime support in `rust-axum`, `openapi`, and
  `typescript`
- example outputs such as `xidlc-examples/api/city_http_stream_openapi.json`

Today those pieces are not yet treated as one stable contract. In particular,
generated OpenAPI still emits `3.1.0`, while the latest official OpenAPI
release is `3.2.0` and explicitly adds first-class support for streaming media
types and `itemSchema`. This change needs to align code generation targets with
the HTTP stream RFC without expanding scope into a separate WebSocket binding
or broader transport redesign.

## Goals / Non-Goals

**Goals:**
- Define one supported HTTP stream subset for generated Rust Axum, OpenAPI, and
  TypeScript outputs.
- Upgrade generated OpenAPI documents to 3.2.0 so stream operations can use
  native streaming descriptions instead of 3.1-era approximations.
- Keep stream method validation consistent across targets, especially around
  allowed codecs and HTTP method constraints.
- Add focused examples and tests that keep the three projections aligned.

**Non-Goals:**
- Redesign the HTTP stream RFC itself.
- Standardize bidirectional HTTP streaming under this change.
- Add new runtime auth middleware, resumable streams, or reliability features.
- Rework non-stream unary HTTP generation beyond what is required for shared
  OpenAPI 3.2.0 plumbing.

## Decisions

### 1. Treat this as a target-alignment change, not a brand new feature line

Decision:
- Build on the existing stream metadata and experimental generators instead of
  introducing a second implementation path.

Rationale:
- The repository already parses and partially emits stream operations for all
  three targets.
- Reusing those code paths keeps examples, snapshots, and runtime helpers
  convergent instead of creating parallel behavior.

Alternatives considered:
- Reimplement stream support from scratch.
  Rejected because it would duplicate existing logic and increase drift risk.

### 2. Upgrade OpenAPI output directly to 3.2.0

Decision:
- Change generated OpenAPI documents to emit `openapi: 3.2.0` and use 3.2
  stream modeling, especially `itemSchema`, for HTTP stream media types.

Rationale:
- OpenAPI 3.2.0 is the first official version that makes stream payloads a
  first-class contract rather than an approximation.
- The HTTP stream RFC already expects OpenAPI generators to describe stream
  item types directly.

Alternatives considered:
- Keep OpenAPI 3.1.0 and document stream bodies as plain schemas.
  Rejected because it preserves an avoidable mismatch now that 3.2.0 is
  available.
- Add vendor extensions on top of 3.1.0.
  Rejected because they would create a transitional contract that users would
  need to unlearn later.

### 3. Keep the supported stream matrix intentionally narrow

Decision:
- Preserve the current target support envelope:
  server-stream uses SSE, client-stream uses NDJSON, and unsupported
  combinations fail code generation with clear errors.

Rationale:
- That matrix already matches the current codebase and is implementable across
  Axum, OpenAPI, and TypeScript.
- Expanding codec combinations in the same change would blur whether failures
  come from RFC alignment or new transport work.

Alternatives considered:
- Generalize all targets to every RFC codec combination immediately.
  Rejected because runtime and client helper changes would broaden scope
  substantially.

### 4. Separate target-specific projection rules from shared stream validation

Decision:
- Keep shared validation in compiler-side stream metadata analysis, then let
  each target project the normalized method into its own output form.

Rationale:
- Axum, OpenAPI, and TypeScript need different emitted shapes, but they should
  agree on stream kind, media types, path/query binding rules, and supported
  method constraints.
- Centralized validation reduces the chance that one target accepts an IDL that
  another rejects.

Alternatives considered:
- Let each generator validate streams independently.
  Rejected because the current drift risk comes from target-specific
  interpretation.

## Risks / Trade-offs

- [Risk] OpenAPI 3.2.0 output may change snapshots and downstream tooling
  expectations. -> Mitigation: update example artifacts and add focused output
  regression coverage.
- [Risk] The RFC mentions stream headers and lifecycle details that are not yet
  enforced everywhere in runtime code. -> Mitigation: specify only the behavior
  this change intends to make normative for the three selected targets.
- [Risk] Existing experimental bidi behavior could be mistaken as part of the
  supported HTTP stream contract. -> Mitigation: keep bidi explicitly out of
  scope in specs and design.
- [Risk] TypeScript stream helpers depend on Web Fetch stream support. ->
  Mitigation: document the runtime assumption and test generated helpers
  against the browser-style fetch contract they already target.

## Migration Plan

1. Update the OpenAPI generator and related snapshots/examples to emit 3.2.0
   stream descriptions.
2. Align Rust Axum and TypeScript generators with the same validated
   server-stream/client-stream subset.
3. Refresh example outputs so the repository demonstrates one coherent stream
   contract across all three targets.
4. Run focused generator, snapshot, and integration tests covering stream
   examples.

Rollback strategy:
- Revert the source change and regenerated artifacts together if OpenAPI 3.2.0
  output or stream target alignment causes unacceptable compatibility issues.

## Open Questions

- Should OpenAPI 3.2.0 output describe NDJSON streams strictly with
  `application/x-ndjson`, or should the generator also consider a more
  standardized JSON Lines media type in the future?
- Which existing example and snapshot set should become the canonical stream
  regression fixture: `city_http_stream` alone, or a smaller purpose-built
  fixture?
