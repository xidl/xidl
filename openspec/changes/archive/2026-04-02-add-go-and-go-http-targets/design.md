## Context

XIDL already has a proven split between a language-level target and an
HTTP-oriented target:

- `rust` vs `rust-axum`
- `rust-jsonrpc` for JSON-RPC projection
- `openapi` / `openrpc` for documentation-oriented outputs

The `rust-axum` path is the closest reference implementation for:

- unary HTTP route and parameter mapping
- `Consumes` / `Produces` handling
- security inheritance and generated clients
- server-stream / client-stream behavior
- example and snapshot coverage

Go should mirror that architecture rather than inventing a one-off layout.
However, unlike Rust Axum, Go should not hard-code a framework dependency at the
center of the target. The RFC semantics are transport-level; `gin` is only one
possible adapter surface.

## Goals / Non-Goals

**Goals:**

- Add a `go` target for plain Go data/model projection.
- Add a `go-http` target that is the Go counterpart to `rust-axum` in scope.
- Use the Go standard library `net/http` as the primary runtime surface so the
  generated target is framework-neutral.
- Keep request/response shaping, security semantics, and stream behavior aligned
  with `docs/rfc/http.md`, `docs/rfc/http-security.md`, and
  `docs/rfc/http-stream.md`.
- Add a Go example suite and integrate it into repository testing.
- Define a file/package layout that supports later adapters such as `gin`
  without rewriting the generator model.

**Non-Goals:**

- Add a Go JSON-RPC target in the same change.
- Guarantee every `rust-axum` convenience helper is mirrored exactly in the
  first Go release.
- Add framework-specific adapters such as `gin` as a hard dependency of the core
  runtime.
- Solve Go module publishing and versioning strategy beyond what is needed for
  in-repo integration.

## Decisions

### 1. Split the Go projection into `go` and `go-http`

Decision:

- Add two generator targets:
  - `go`: plain Go types, enums, aliases, structs, unions-as-best-effort, and
    interface signatures where transport mapping is not required
  - `go-http`: HTTP-specific generated client/server surfaces and helper types

Rationale:

- This mirrors the existing `rust` / `rust-axum` split that already works well.
- It keeps transport-independent type generation usable outside HTTP.
- It prevents HTTP runtime concerns from leaking into plain generated models.

Alternatives considered:

- One combined `go` target that always emits HTTP helpers. Rejected because it
  conflates language projection with transport projection.
- Only `go-http`, no plain `go`. Rejected because non-HTTP consumers still need
  Go model generation.

### 2. Make `go-http` framework-neutral and build on `net/http`

Decision:

- The first `go-http` runtime will target standard-library `net/http`.
- Generated server registration will expose `http.Handler` or `*http.ServeMux`
  friendly surfaces.
- Framework-specific adapters, including `gin`, stay outside the core target and
  can be added later as optional packages or example adapters.

Rationale:

- `net/http` is the lowest common denominator in Go.
- It avoids binding the target contract to a single routing or middleware
  framework.
- It keeps generated code reusable across `gin`, `chi`, `echo`, and plain stdlib
  applications.

Alternatives considered:

- Use `gin` directly for the first release. Rejected because it would make
  routing, context, middleware, and testing behavior framework-specific.
- Generate only interfaces and leave all HTTP glue to user code. Rejected
  because it would not match the value of `rust-axum`.

### 3. Introduce two Go workspace packages: `xidl-go` and `xidl-go-http`

Decision:

- Add:
  - `xidl-go`: shared language/runtime helpers used by generated `go` output
  - `xidl-go-http`: HTTP runtime helpers used by generated `go-http` output

Rationale:

- This mirrors the Rust runtime package arrangement and keeps the generator
  dependency story explicit.
- Shared helper code avoids duplicating response encoding, media-type checks,
  auth helpers, stream framing, and error handling across generated files.

Alternatives considered:

- Put all helpers into one `xidl-go-http` package. Rejected because plain `go`
  generation would then pick up HTTP concerns.
- Generate everything self-contained with no runtime package. Rejected because
  it would explode template complexity and make bug fixes harder.

### 4. Keep the first `go-http` release RFC-aligned, but intentionally narrower

than `rust-axum` in convenience APIs

Decision:

- Match `rust-axum` behavior for RFC-visible semantics:
  - route/path/query/header/cookie/body shaping
  - request and response media types
  - security inheritance and generated client auth
  - HTTP stream profile support
- Allow differences in convenience-layer API shape where Go idioms differ:
  - explicit `context.Context`
  - `http.Handler` registration
  - `error` returns instead of Rust-style `Result`

Rationale:

- Consumers need semantic equivalence, not syntax equivalence.
- Go users expect idiomatic interfaces and `context.Context`.

Alternatives considered:

- Copy the Rust API surface mechanically. Rejected because it would feel foreign
  in Go and increase maintenance burden.

### 5. Use generated package layout based on file stem, not build tags, for the

primary integration model

Decision:

- Generated Go code will be organized by package/file output, similar to other
  targets.
- `//go:build` will not be the primary mechanism for splitting `go` vs `go-http`
  or client vs server output.

Rationale:

- Build tags hide API shape behind compile conditions and make snapshot and
  example coverage harder to reason about.
- XIDL already models target variants explicitly through generators and
  properties; Go should stay aligned with that.
- Package-level separation is clearer for generated consumers and CI.

Where `//go:build` is appropriate:

- optional adapter packages, e.g. `gin`
- example binaries gated by transport or environment
- CI-only integration lanes

Conclusion:

- Use package boundaries for core generated outputs.
- Reserve build tags for optional integrations, not the base target split.

### 6. Add `xidlc-examples-go` as a dedicated sibling to `xidlc-examples`

Decision:

- Create a new workspace member `xidlc-examples-go` rather than folding Go
  examples into the existing Rust examples crate.

Rationale:

- Go has a different module, toolchain, and test runner.
- A dedicated directory keeps `go.mod`, fixtures, generated output, and tests
  isolated and easier to run in CI.

Alternatives considered:

- Reuse `xidlc-examples` with ad-hoc shelling out to Go. Rejected because it
  mixes ecosystems and complicates build/test ownership.

## Proposed Architecture

### 1. Generator targets

Add new generator namespaces:

- `xidlc/src/generate/go/`
- `xidlc/src/generate/go_http/`

Expected shape:

- `mod.rs`
- `definition.rs`
- `interface.rs`
- `render.rs`
- `spec.rs`
- `templates/*.j2`

Generator responsibilities:

- `go`
  - emit package declarations
  - emit structs/enums/aliases/request types
  - optionally emit interface signatures for declarations
  - avoid HTTP runtime coupling
- `go-http`
  - reuse normalized HTTP/security/stream metadata from shared utils
  - emit client and server code
  - emit helper request/response structs and auth wrappers
  - depend on `xidl-go-http`

### 2. Runtime packages

Add workspace members:

- `xidl-go`
- `xidl-go-http`

`xidl-go` initial scope:

- basic shared types if generated output needs them
- small helpers for optional values or enum/string conversion
- minimal by default

`xidl-go-http` initial scope:

- request wrapper types
- HTTP error type and encoding
- media-type helpers
- serialization factory / deserialization factory
- auth extraction and client auth helpers
- stream framing helpers:
  - SSE for server-stream
  - NDJSON for client-stream and shared framing

### 3. Server shape

Generated server code should center on a Go interface and registration helper:

- generated interface implemented by user code
- generated `Register<Service>` function wiring handlers to `http.ServeMux` or a
  provided registration surface
- handlers decode inputs, enforce media types, apply security extraction, call
  service methods, and encode responses

Preferred shape:

- `func RegisterUserAPI(mux *http.ServeMux, svc UserAPI)`
- handler functions using `http.HandlerFunc`

Possible extension point:

- expose lower-level generated handler constructors so adapters like `gin` can
  wrap them later

### 4. Client shape

Generated clients should use:

- a base URL
- an underlying `*http.Client`
- generated methods per endpoint
- auth configuration similar in spirit to `xidl-rust-axum::ClientAuth`

Expected conventions:

- generated constructor with default `http.Client`
- generated constructor with injected `*http.Client`
- method signatures accept request-side business parameters directly and return:
  - response type, `error`
  - stream reader/writer helpers where applicable

### 5. Media type handling

Unary `go-http` should support the same baseline media types now supported by
`rust-axum`:

- `application/json`
- `application/x-www-form-urlencoded`
- `application/msgpack` (gated by runtime feature/package support)

Design direction:

- keep MIME-to-codec dispatch in `xidl-go-http`
- do not inline codec logic into every generated method

### 6. Security mapping

`go-http` should implement the RFC-aligned subset already covered by
`rust-axum`:

- `@http_basic`
- `@http_bearer`
- `@api_key(...)`
- `@no_security`
- inherited interface-level security and method-level overrides

Server runtime:

- extract auth before invoking user handlers
- carry extracted auth in generated request wrapper structs or explicit helper
  fields

Client runtime:

- structured auth configuration object
- automatic application of auth headers / query / cookies based on generated
  method requirements

### 7. Stream mapping

Initial `go-http` stream scope should mirror the current supported matrix:

- `@server_stream` with SSE
- `@client_stream` with NDJSON
- no bidi HTTP stream target in the first release

Runtime direction:

- SSE writer helpers for server-stream
- NDJSON reader/writer helpers for client-stream
- shared error framing aligned with the HTTP stream RFC profile

### 8. Test layout

#### `xidlc/tests`

Add new snapshot input folder:

- `xidlc/tests/go/`

Add snapshot coverage for:

- unary HTTP mapping
- security inheritance
- stream method projection
- media-type generation

Potential next step:

- if output surface grows large, split into `go/` and `go_http/` fixture naming
  conventions while still using one snapshot folder

#### `xidlc-examples-go`

Add:

- `api/http/*.idl`
- generated outputs as needed
- Go tests using `go test`

Coverage goals:

- unary HTTP
- security-protected endpoints
- server-stream / client-stream examples
- media-type examples

#### Rust-side integration harness

Do not try to run Go tests inside `xidlc` snapshot tests directly.

Instead:

- keep codegen coverage in `xidlc/tests`
- keep executable Go integration tests in `xidlc-examples-go`
- add CI commands that run both Rust and Go test suites

## Risks / Trade-offs

- [Risk] Go generator scope expands too quickly if unary, security, and stream
  support all land at once. -> Mitigation: phase implementation behind the task
  plan and keep the first runtime framework-neutral.
- [Risk] Attempting to support `gin` directly would create framework lock-in. ->
  Mitigation: standardize on `net/http` first and design adapter seams.
- [Risk] Build tags could obscure generated surface and make tests flaky. ->
  Mitigation: use package boundaries for core targets and reserve build tags for
  optional adapters only.
- [Risk] Go-specific idioms may drift from RFC semantics if modeled too loosely.
  -> Mitigation: keep shared validation in compiler utilities and only vary
  emitted API shape where idiomatic Go requires it.
- [Risk] Msgpack and streaming helpers introduce third-party runtime
  dependencies. -> Mitigation: keep dependency boundaries explicit in
  `xidl-go-http` and gate optional codecs or adapters conservatively.

## Migration Plan

1. Add design and task scaffolding for `go` and `go-http`.
2. Land `go` language projection and snapshot coverage.
3. Land `xidl-go-http` runtime and unary `go-http` generation.
4. Add security-aware client/server behavior.
5. Add stream support and complete the supported RFC subset.
6. Add `xidlc-examples-go` and wire CI/test commands.

Rollback strategy:

- revert the new targets and workspace members together
- keep shared validation changes isolated where possible so they can be reverted
  independently if needed

## Open Questions

- Should `go` generate interface declarations for transport-oriented methods, or
  should those live only in `go-http`?
- Should `application/msgpack` be enabled by default in `xidl-go-http`, or
  should it be optional via module/package separation?
- Do we want the first post-stdlib adapter to be `gin`, `chi`, or none until
  demand is clearer?
- How much of the Rust example matrix should be mirrored one-for-one in
  `xidlc-examples-go`, versus adding a smaller purpose-built Go suite first?
