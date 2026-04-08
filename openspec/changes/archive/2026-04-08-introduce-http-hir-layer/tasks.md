## 1. HTTP projection pipeline

- [x] 1.1 Add a dedicated `http-hir` projection stage and data model under
      `xidlc/src/generate/` that extends base HIR with normalized HTTP operation
      metadata and HTTP document metadata.
- [x] 1.2 Extend `xidlc/src/jsonrpc/` and related codegen protocol types with a
      dedicated `http_hir` artifact kind.
- [x] 1.3 Update `xidlc/src/driver/generate.rs` to route `rust-axum`, `go-http`,
      `python-http`, and `openapi` through `hir -> http-hir`, while keeping
      generic `hir` for all other targets.
- [x] 1.4 Define a stable handoff artifact from `http-hir` to downstream
      generators so renderers receive shared HTTP metadata instead of raw RFC
      annotation interpretation work.

## 2. Shared RFC interpretation

- [x] 2.1 Move route normalization, route-template parsing, default method and
      parameter-source resolution, and request/response body shaping into the
      shared `http-hir` projection.
- [x] 2.2 Move shared HTTP security and stream interpretation into `http-hir`,
      including inheritance, effective media types, and RFC-level validation.
- [x] 2.3 Project attribute-derived effective HTTP operations into `http-hir` so
      renderers do not carry separate attribute transport rules.
- [x] 2.4 Move OpenAPI-facing pragma metadata such as package, version, and
      service/server definitions into `http-hir`.
- [x] 2.5 Add projection-level tests that assert normalized `http-hir` output
      for representative unary, attribute, security, stream, and pragma-driven
      fixtures.

## 3. Renderer migration

- [x] 3.1 Refactor `rust-axum` to consume `http-hir` and remove duplicated
      target-local HTTP parsing where semantics are now shared.
- [x] 3.2 Refactor `go-http` to consume `http-hir` and remove duplicated
      target-local HTTP parsing where semantics are now shared.
- [x] 3.3 Refactor `python-http` to consume `http-hir` and remove duplicated
      target-local HTTP parsing where semantics are now shared.
- [x] 3.4 Refactor `openapi` to consume `http-hir` and remove duplicated
      target-local HTTP parsing and pragma interpretation where semantics are
      now shared.
- [x] 3.5 Remove or internalize `xidlc/src/generate/utils/http.rs` so HTTP
      semantics only live under the `http-hir` implementation.

## 4. Regression coverage

- [x] 4.1 Refresh or add generator snapshot coverage proving that shared HTTP
      fixtures produce consistent `rust-axum`, `go-http`, `python-http`, and
      `openapi` behavior after the migration.
- [x] 4.2 Add validation coverage showing RFC errors now originate from the
      shared `http-hir` layer instead of one target-specific renderer only.
