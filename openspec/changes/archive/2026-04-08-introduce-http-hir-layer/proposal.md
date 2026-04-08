## Why

`xidlc` currently applies the HTTP RFCs in multiple target-specific generators,
so route resolution, parameter binding, security inheritance, stream handling,
OpenAPI-facing document metadata, and validation are implemented more than once.
The duplicated logic in `rust-axum`, `go-http`, `python-http`, and `openapi` has
already drifted in small but important ways, which makes the repository's HTTP
behavior harder to reason about and increases the cost of evolving
`docs/rfc/http*.md`.

## What Changes

- Introduce a shared `http-hir` projection layer in `xidlc` that extends the
  existing HIR with HTTP-specific, RFC-normalized semantic metadata and HTTP
  document metadata needed by downstream generators.
- Extend the codegen RPC/artifact protocol with a dedicated `http_hir` kind so
  the pipeline can pass a first-class HTTP projection instead of overloading
  generic HIR artifacts.
- Move HTTP RFC interpretation out of `rust-axum`, `go-http`, `python-http`, and
  `openapi`, so those generators consume `http-hir` and focus on rendering
  target code or documents instead of reparsing annotations and pragmas.
- Update the pipeline so HTTP RFC-backed targets route through
  `source -> hir -> http-hir -> target`, while non-HTTP targets continue to use
  the current `source -> hir -> target` path unchanged.
- Centralize HTTP route normalization, parameter-source resolution,
  request/response shaping, media-type resolution, security inheritance, stream
  validation, attribute-derived effective operations, and OpenAPI-facing pragma
  metadata behind the shared projection.
- Add targeted coverage proving that the shared `http-hir` output drives
  consistent behavior across the supported HTTP generators.

## Capabilities

### New Capabilities

- `http-hir-projection`: Build one shared HTTP-focused intermediate
  representation from HIR and use it as the single HTTP RFC interpretation layer
  for supported HTTP generators.

### Modified Capabilities

## Impact

- Affected code: `xidlc/src/driver/generate.rs`, `xidlc/src/jsonrpc/`, a new
  `http-hir` generator stage/module, and the HTTP generators under
  `xidlc/src/generate/`.
- Affected targets: `rust-axum`, `go-http`, `python-http`, and `openapi`.
- Affected behavior: HTTP RFC interpretation becomes centralized, which reduces
  drift between targets and makes future RFC updates apply from one place.
- Risks: preserving current generated output shapes while moving logic,
  extending the RPC artifact protocol safely, and defining a `http-hir` model
  that is shared enough to remove duplication without becoming target-specific
  again.
