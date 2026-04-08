# http-hir-projection Specification

## Purpose
TBD - created by archiving change introduce-http-hir-layer. Update Purpose after archive.
## Requirements
### Requirement: HTTP Target Selection Uses HTTP HIR

The system MUST select a dedicated `http-hir` projection stage for supported
HTTP generators and pass it as a first-class artifact instead of always routing
source text through the generic `hir` stage only.

#### Scenario: HTTP generator dispatch selects http-hir

- **WHEN** `xidlc` generates code for `rust-axum`, `go-http`, `python-http`, or
  `openapi`
- **THEN** the driver MUST route the compilation pipeline through `hir` and then
  `http-hir` before invoking the target renderer

#### Scenario: HTTP projection is passed as a dedicated artifact

- **WHEN** the `http-hir` projection stage hands off work to a downstream HTTP
  renderer
- **THEN** the codegen protocol MUST use a dedicated `http_hir` artifact kind
  rather than encoding the projection as generic HIR or renderer-local props

#### Scenario: Non-HTTP generator dispatch keeps generic hir

- **WHEN** `xidlc` generates code for a target outside the supported HTTP set
- **THEN** the driver MUST continue to route the compilation pipeline through
  the existing generic `hir` stage

### Requirement: HTTP HIR Normalizes RFC Semantics Once

The system MUST interpret `docs/rfc/http.md`, `docs/rfc/http-security.md`, and
`docs/rfc/http-stream.md` in one shared projection layer and expose the
normalized result as `http-hir`.

#### Scenario: Route and binding semantics are resolved in http-hir

- **WHEN** an HTTP interface uses verb annotations, route templates, path/query
  inference, media-type annotations, or request/response shaping annotations
- **THEN** `http-hir` MUST emit normalized operation metadata for effective
  routes, parameter sources, body shaping, and media types without requiring
  renderers to reparse those annotations

#### Scenario: Security and stream semantics are resolved in http-hir

- **WHEN** an HTTP interface uses security inheritance, stream annotations,
  stream codecs, or RFC-level validation rules
- **THEN** `http-hir` MUST emit normalized security and stream metadata together
  with any shared RFC validation outcomes before renderer execution

#### Scenario: Attribute-derived operations are resolved in http-hir

- **WHEN** an interface contains attributes that map to effective HTTP
  operations for supported targets
- **THEN** `http-hir` MUST emit those effective operations with the same shared
  route, security, stream, and response-shaping semantics used for method-based
  operations

#### Scenario: OpenAPI-facing pragma metadata is resolved in http-hir

- **WHEN** a specification includes HTTP-relevant `#pragma xidlc` metadata such
  as package, version, or service definitions
- **THEN** `http-hir` MUST emit normalized HTTP document metadata so downstream
  generators do not reparse those pragmas independently

### Requirement: HTTP Renderers Consume Shared HTTP HIR

Supported HTTP generators MUST render from `http-hir` instead of each
independently re-deriving HTTP semantics from parser HIR annotations.

#### Scenario: Rust Axum renders from shared http-hir

- **WHEN** the `rust-axum` target renders an HTTP interface
- **THEN** route resolution, parameter binding, security inheritance, and stream
  interpretation MUST come from `http-hir` rather than target-local annotation
  parsing

#### Scenario: Go HTTP renders from shared http-hir

- **WHEN** the `go-http` target renders an HTTP interface
- **THEN** route resolution, parameter binding, security inheritance, and stream
  interpretation MUST come from `http-hir` rather than target-local annotation
  parsing

#### Scenario: Python HTTP renders from shared http-hir

- **WHEN** the `python-http` target renders an HTTP interface
- **THEN** route resolution, parameter binding, security inheritance, and stream
  interpretation MUST come from `http-hir` rather than target-local annotation
  parsing

#### Scenario: OpenAPI renders from shared http-hir

- **WHEN** the `openapi` target renders an HTTP specification
- **THEN** route resolution, parameter binding, security inheritance, stream
  interpretation, attribute-derived operations, and HTTP document metadata MUST
  come from `http-hir` rather than target-local annotation or pragma parsing

### Requirement: Shared HTTP Projection Is Regression Tested

The system MUST include coverage that proves supported HTTP generators share one
RFC interpretation path.

#### Scenario: Projection fixtures assert normalized http-hir output

- **WHEN** repository tests exercise HTTP RFC fixtures with overlapping route,
  attribute, security, stream, and pragma-driven metadata features
- **THEN** the test suite MUST assert the normalized `http-hir` output directly
  so RFC interpretation regressions are caught before renderer-specific
  snapshots diverge

#### Scenario: Renderer snapshots stay aligned with shared projection

- **WHEN** repository tests generate `rust-axum`, `go-http`, `python-http`, and
  `openapi` output from the same HTTP fixtures
- **THEN** the test suite MUST verify those renderers continue to produce output
  consistent with the shared `http-hir` semantics

