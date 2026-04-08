## Context

`xidlc/src/driver/generate.rs` currently hard-codes the first projection step as
`source -> hir` by calling `generate_for_lang("hir", ...)`. The shared HIR stage
in `xidlc/src/generate/hir_gen/mod.rs` parses source text, builds the standard
parser HIR, and immediately forwards that HIR to the requested target.

That works well for language targets that directly render the parser HIR, but it
breaks down for HTTP generators because the repository now has multiple
target-local HTTP interpreters. `go_http/definition.rs` owns route parsing,
default source resolution, and route-template analysis. `python_http/spec.rs`
reimplements HTTP method selection, default path generation, route validation,
parameter binding, and stream-aware constraints. `rust_axum/interface.rs`
contains another copy of route parsing, path normalization, source binding, HEAD
validation, security extraction, attribute-derived operations, and
stream-specific defaults. `openapi/mod.rs` implements another large slice of
HTTP interpretation, including route binding, response shaping, stream patches,
security, and pragma-derived service metadata. These implementations aim at the
same RFCs in `docs/rfc/http.md`, `docs/rfc/http-security.md`, and
`docs/rfc/http-stream.md`, but they are no longer guaranteed to evolve together.

The requested change is architectural rather than cosmetic: add a dedicated
`http-hir` layer that extends HIR with HTTP-ready semantic metadata, add a
dedicated RPC artifact kind for it, then make HTTP targets render from that
shared projection instead of each target parsing annotations or pragmas for
itself.

## Goals / Non-Goals

**Goals:**

- Add a shared `http-hir` projection stage that translates parser HIR into an
  HTTP-focused intermediate model aligned with `docs/rfc/http*.md`.
- Extend the RPC/artifact protocol so `http-hir` is a first-class artifact
  rather than an implicit convention layered on top of generic HIR payloads.
- Change generator dispatch so supported HTTP targets route through
  `source -> hir -> http-hir -> target`, while non-HTTP targets keep
  `source -> hir -> target`.
- Ensure `rust-axum`, `go-http`, `python-http`, and `openapi` consume the same
  normalized HTTP semantics for routes, parameter sources, media types,
  security, stream metadata, and HTTP document metadata.
- Project both method-based and attribute-derived effective HTTP operations so
  renderers do not need separate transport rules for attributes.
- Keep target generators responsible for rendering and target-runtime concerns,
  not RFC interpretation.
- Retire `xidlc/src/generate/utils/http.rs` as the home of HTTP semantics by
  moving its surviving logic into the `http-hir` implementation.
- Preserve current externally visible behavior where the existing generators are
  already aligned with the RFCs, and add regression coverage around the shared
  projection.

**Non-Goals:**

- Do not redesign the base parser HIR used by non-HTTP targets.
- Do not redefine the HTTP RFCs themselves; this work implements and centralizes
  the current RFC intent.
- Do not force every renderer to share identical emitted helper types or runtime
  APIs; only the normalized HTTP semantic input must be shared.
- Do not force all target-specific capability limits into `http-hir`; renderer
  support checks that are not RFC semantics may remain target-local.

## Decisions

### 1. Add `http-hir` as a separate projection stage instead of mutating base HIR

The new layer should be implemented as an extension/projection of HIR rather
than by embedding HTTP-only fields into the base parser HIR. The base HIR is a
transport-neutral representation consumed by many targets; pushing HTTP-specific
route, stream, and security semantics into it would blur that boundary and make
non-HTTP targets pay for concepts they do not use.

The alternative was to add resolved HTTP metadata directly onto parser HIR
nodes. That would couple transport-specific interpretation to the generic AST
conversion layer and make future non-HTTP projections harder to separate, so it
was not chosen.

### 2. Add a first-class `http_hir` artifact to the RPC/codegen protocol

The current protocol only passes generic HIR artifacts between stages. To make
`http-hir` a real projection stage rather than an implicit helper call, the RPC
and artifact model should add a dedicated `http_hir` kind. That payload should
carry the base parser HIR relationship plus resolved HTTP semantics in one
stable handoff object.

The alternative was to encode `http-hir` indirectly inside generic HIR props or
to leave it as an in-process helper call. That would keep the boundary
ambiguous, make downstream typing weaker, and blur whether a renderer is truly
consuming shared semantics, so it was not chosen.

### 3. Switch dispatch to `source -> hir -> http-hir -> target` for HTTP targets

`xidlc/src/driver/generate.rs` already isolates the first projection step by
calling the `"hir"` plugin before continuing to the final generator. That is the
correct insertion point for the new branch: when the requested language is one
of the supported HTTP targets, the driver should first build parser HIR, then
call `http-hir`, then continue to the final renderer. Non-HTTP targets should
keep the current path.

The alternative was to let each HTTP target call back into a shared helper after
receiving ordinary HIR. That would still centralize some code, but it would not
change the pipeline shape and would keep the HTTP projection contract implicit
inside renderers, so it was not chosen.

### 4. Make `http-hir` own normalized HTTP operation metadata end to end

`http-hir` should produce one normalized model per effective HTTP operation,
including both operations declared directly from methods and operations derived
from IDL attributes where current targets expose them. The projection should
include:

- effective HTTP method and normalized route set
- parsed route template metadata
- request-side and response-side parameter partitions
- resolved request/response media types
- resolved HTTP security requirements and origin
- resolved stream mode, codec, and target-independent validation outcomes
- RFC-derived flags such as flattening, optionality, deprecation, status/body
  shaping, and HEAD constraints where applicable
- HTTP document metadata required by downstream generators such as package,
  version, and service/server pragma information

This gives renderers a stable semantic input and removes target-local annotation
and pragma parsing as a source of drift.

The alternative was to keep `http-hir` narrowly focused on routes and parameter
sources only. That would leave security, stream, response-shape, attribute, and
OpenAPI metadata logic duplicated across renderers, which fails the goal of
making HTTP RFC interpretation consistent, so it was not chosen.

### 5. Keep renderer-owned logic limited to target syntax and runtime adaptation

After the projection is introduced, `rust-axum`, `go-http`, `python-http`, and
`openapi` should treat `http-hir` as their semantic input. They may still apply
target-specific naming, helper emission, runtime glue, schema/document
formatting, or unsupported-target checks that are inherently renderer-specific,
but they should not re-derive routes, binding sources, security inheritance,
stream semantics, attribute transport behavior, or pragma-derived HTTP metadata
from raw HIR annotations.

The alternative was to allow partial renderer re-interpretation "when
convenient." That would recreate the current fragmentation and make future RFC
fixes ambiguous, so it was not chosen.

### 6. Preserve base HIR for non-HTTP targets and make HTTP target selection explicit

The first version should explicitly map `rust-axum`, `go-http`, `python-http`,
and `openapi` to `http-hir`, and leave all other targets on `hir`. This keeps
rollout risk contained and makes it easy to audit which generators depend on the
new stage.

The alternative was to infer "HTTP-ness" indirectly or switch every
HTTP-adjacent generator immediately. That would make rollout harder to reason
about and could pull unrelated generators into a partially designed projection,
so it was not chosen.

### 7. Retire `generate/utils/http.rs` as the semantic source of truth

The repository already has a partial shared helper layer in
`xidlc/src/generate/utils/http.rs`. After `http-hir` exists, HTTP semantics
should live in one place only. Shared logic that remains useful should move
under the `http-hir` implementation, and renderers should no longer import HTTP
RFC interpretation helpers from `generate/utils/http.rs`.

The alternative was to keep both `utils/http.rs` and `http-hir` as parallel
semantic layers. That would preserve ambiguity about which layer owns RFC
behavior, so it was not chosen.

### 8. Add shared projection tests before or alongside renderer migration

The test strategy should validate both the new projection and the renderer
integration:

- projection-level fixtures asserting normalized routes, bindings, security,
  stream metadata, attribute-derived operations, and HTTP document metadata
- regression tests for `rust-axum`, `go-http`, `python-http`, and `openapi`
  snapshots to ensure generated output remains stable where behavior is
  intentionally unchanged
- targeted validation cases proving RFC errors are raised from the shared layer
  instead of only from a single renderer

The alternative was to rely only on downstream snapshots. That would miss
whether semantic drift has merely moved from one renderer to the shared layer,
so it was not chosen.

## Risks / Trade-offs

- [The `http-hir` model becomes too target-specific] -> Define projection types
  around RFC semantics rather than renderer helper structures, and keep naming
  or runtime wrappers in renderer code.
- [The RPC/artifact boundary becomes harder to evolve] -> Add `http_hir` as an
  explicit typed artifact instead of hiding it inside generic HIR props.
- [Migration changes generated output even when RFC behavior is intended to stay
  the same] -> Add projection-level assertions and keep renderer snapshot tests
  during the migration.
- [Some existing renderer validations do not fit cleanly into shared projection]
  -> Move RFC-derived validation into `http-hir` and keep only truly
  target-support checks in renderers.
- [Target selection rules drift as more HTTP generators appear] -> Centralize
  the list of `http-hir` targets in the driver instead of scattering the
  decision across generators.

## Migration Plan

Implement the new projection stage and `http_hir` artifact first, wire the
driver to select it for the initial HTTP target set, and migrate the HTTP
generators one by one to consume `http-hir` while preserving existing snapshots
where behavior is unchanged. During rollout, keep projection-level and
target-level tests together so it is clear whether a regression comes from the
shared model or the renderer. As the renderers migrate, move or delete
`generate/utils/http.rs` helpers so the semantic ownership is unambiguous.

If rollback is required, the driver can point the affected targets back to the
existing `hir` stage while preserving the partially implemented `http-hir`
module behind feature-gated or unused code until the migration is retried.

## Open Questions

- What the concrete `http_hir` payload shape should be so it stays close enough
  to HIR for reuse without encouraging renderers to bypass normalized fields.
- How much OpenAPI-specific document shaping should remain in `openapi` after
  `http-hir` owns shared HTTP semantics and pragma-derived metadata.
- Whether any non-RFC renderer limitations currently expressed as validation
  errors should become richer capability reporting instead of simple migration
  of existing checks.
