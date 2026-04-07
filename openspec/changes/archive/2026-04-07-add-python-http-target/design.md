## Context

The repository already has two reusable target patterns:

- `rust` / `rust-axum`, which splits language-level generation from HTTP
  generation and keeps the runtime in a dedicated crate.
- `go` / `go-http`, which uses the same split and protects RFC-visible behavior
  through snapshot, validation, and end-to-end integration tests.

This change needs to establish the same capability for Python and align it
explicitly with `docs/rfc/http.md`, `docs/rfc/http-security.md`, and
`docs/rfc/http-stream.md`. The user also requires the runtime to live under the
top-level `python/` directory, to use `minijinja` for rendering, and to study
`rust-axum` before implementation.

## Goals / Non-Goals

**Goals:**

- Add `python` and `python-http` generator targets that follow the existing
  target split.
- Implement the repository-supported subset of the unary HTTP, HTTP security,
  and HTTP stream RFCs in `python-http`.
- Add a Python runtime under `python/` for shared model and HTTP support used by
  generated code.
- Keep the server-side runtime framework-neutral while making the generated
  contract straightforward to integrate into Django and FastAPI.
- Generate abstract service interfaces plus route-registration helpers so that
  subclass implementations can be mounted automatically, similar to the
  `rust-axum` service-and-router pattern.
- Use `minijinja` as the Python code generation engine, consistent with other
  templated targets.
- Add Python snapshot, validation, and integration coverage modeled after the
  existing Go workflow.

**Non-Goals:**

- Add Python JSON-RPC support in the same change.
- Bind the initial Python HTTP target to FastAPI, Starlette, Django, or any
  other specific framework.
- Implement bidirectional HTTP streaming.
- Solve Python packaging, PyPI publishing, or external dependency distribution.

## Decisions

### 1. Split Python support into `python` and `python-http`

Decision:

- `python` owns language-level projection such as dataclasses, enums, aliases,
  and base interface shapes.
- `python-http` owns async server bindings, stream helper structures, and
  security metadata.

Rationale:

- This matches the working pattern already used by `rust` / `rust-axum` and `go`
  / `go-http`.
- It keeps plain model generation free of HTTP runtime concerns.
- It leaves room for future non-HTTP Python projections without polluting the
  base target.

Alternatives considered:

- A single `python` target that always emits HTTP helpers. Rejected because it
  conflates language and transport responsibilities.

### 2. Keep the Python runtime under the top-level `python/` directory

Decision:

- Add a new `python/` directory as the runtime root for generated Python code.
- Split ordinary model support and HTTP-specific support by module, but keep
  them under the same Python root.

Rationale:

- This is an explicit user requirement.
- It serves the same role as the Rust and Go runtime packages while fitting the
  repository layout requested for Python.
- It lets integration tests import the in-repo runtime directly without solving
  publishing first.

Alternatives considered:

- Emit all runtime helpers into every generated file. Rejected because it would
  bloat templates and make bug fixes hard to centralize.

### 3. Keep RFC semantics in shared HTTP utilities

Decision:

- Route resolution, parameter-source rules, effective security requirements, and
  stream constraints should continue to live in or extend
  `generate/utils/http.rs` when they are target-agnostic.
- `python-http` should own only the Python projection layer and emitted runtime
  structures.

Rationale:

- `rust-axum`, `openapi`, `ts`, and `go-http` already depend on that shared
  path.
- Python should not duplicate RFC logic and drift from the rest of the repo.

Alternatives considered:

- Reimplement HTTP, security, and stream rules inside the Python generator.
  Rejected because it duplicates logic and increases long-term maintenance risk.

### 4. Use the same `minijinja` renderer pattern as other generated targets

Decision:

- Add `xidlc/src/generate/python/` and `xidlc/src/generate/python_http/`.
- Use the same renderer pattern as `rust-axum` and `go-http`, backed by
  `include_dir` and `minijinja` templates.

Rationale:

- The repository already has a proven implementation model for template-driven
  targets.
- Python output will contain a large amount of repeated structural code, which
  is better expressed through templates than string concatenation.

Alternatives considered:

- Hand-build Python source strings in Rust. Rejected because templates are
  easier to maintain at this scale.

### 5. Keep the first Python HTTP release framework-neutral

Decision:

- Organize generated output around Python-standard data structures and a small
  repository runtime abstraction rather than a hard dependency on ASGI, WSGI,
  FastAPI, Django, or another framework.
- Generated server code should target request/response abstractions defined in
  the runtime, with framework adapters layered on top.
- The first-class adapter targets for this abstraction are Django and FastAPI,
  but the generated core must not depend on either one directly.

Rationale:

- This mirrors the reasoning behind the first `go-http` release using the
  standard library instead of a framework-specific adapter.
- The priority for this change is RFC alignment and test coverage without
  locking the generated API to one Python web framework.

Alternatives considered:

- Bind directly to FastAPI or Django. Rejected because it would lock the target
  API and dependency model to one framework too early.

### 6. Generate abstract service interfaces with automatic route registration

Decision:

- For each generated HTTP interface, emit an abstract Python service contract
  whose methods represent the effective RPC operations.
- Emit generated route-registration helpers that inspect the service contract
  metadata and register concrete subclass implementations automatically.
- Keep route registration independent from any one framework by targeting a
  small adapter surface that Django and FastAPI integrations can implement.

Rationale:

- This is the closest Python equivalent to the generated `axum` service/router
- pattern and the Go `Service interface + NewHandler(...)` pattern.
- It gives users a stable implementation surface: subclass the generated
  interface, implement the methods, then hand the instance to generated routing
  helpers.
- It centralizes binding, validation, auth extraction, and stream wiring in
  generated code instead of forcing each framework integration to rebuild that
  logic.

Alternatives considered:

- Generate only plain functions and ask users to wire routes manually. Rejected
  because it makes every integration repeat binding and metadata logic.
- Generate one framework-specific base class per framework. Rejected because it
  duplicates logic and weakens framework independence.

### 7. Land tests in three layers: snapshot, validation, and integration

Decision:

- Add focused fixtures under `xidlc/tests/python/` and
  `xidlc/tests/python-http/`.
- Extend `xidlc/tests/http_validation.rs` with Python HTTP invalid-combination
  assertions.
- Add a Python integration test directory and `Makefile` modeled after
  `golang/Makefile`, including representative Django and FastAPI adapter flows.

Rationale:

- This matches the test strategy already used by the Go HTTP target.
- Snapshots protect generated structure, validation tests protect rejected RFC
  combinations, and integration tests protect runtime behavior.

Alternatives considered:

- Add snapshots only. Rejected because that would not prove the generated Python
  runtime actually works end to end.

## Risks / Trade-offs

- `[Risk] Python APIs will not be isomorphic to rust-axum` -> Mitigation: match
  RFC-visible semantics and metadata, not Rust syntax.
- `[Risk] The Python runtime boundary could ossify too early` -> Mitigation:
  keep the runtime focused on transport abstractions and codec helpers instead
  of framework-specific concepts.
- `[Risk] Django and FastAPI may need slightly different request/response
  adapter semantics`
  -> Mitigation: keep the generated registration contract narrow and let
  framework adapters normalize into the shared runtime shape.
- `[Risk] A new target may expose gaps in shared HTTP rules` -> Mitigation: fix
  target-agnostic gaps in `generate/utils/http.rs` first and re-run existing
  target tests.
- `[Risk] Integration tests introduce Python toolchain requirements` ->
  Mitigation: isolate them behind a dedicated directory and `Makefile`,
  mirroring the existing Go workflow.

## Migration Plan

- Add the Python generator targets and the `python/` runtime directory.
- Wire the Python targets into the `xidlc` driver, renderer, and artifact output
  flow.
- Add Python snapshot, validation, and integration coverage alongside existing
  test commands.
- If implementation exposes gaps in shared HTTP behavior, fix the shared layer
  first and then update affected target snapshots.

## Open Questions

- Whether the minimal framework adapter surface should normalize everything into
  async handlers, or support both sync and async service implementations.
  Resolved: the initial implementation supports async-only services.
- Whether the first plain `python` target should emit full interface stubs or
  focus initially on data models and HTTP-adjacent types.
