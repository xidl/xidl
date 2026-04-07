## Why

`xidlc` already has a proven split between language-level generation and
HTTP-oriented generation in `rust`/`rust-axum` and `go`/`go-http`, but it still
lacks an equivalent Python target. The repository now has published RFCs for
HTTP, HTTP security, and HTTP streaming, so Python support should land against
the same contracts and with the same snapshot and integration-test protection
that existing targets already have.

## What Changes

- Add a `python` target for Python data models and interface-adjacent code.
- Add a `python-http` target that projects the HTTP RFCs into generated Python
  async server bindings and metadata.
- Add a Python runtime under `python/` for shared generated-code support,
  including common HTTP, security, and stream helpers.
- Keep the Python HTTP runtime framework-neutral while making it directly
  adaptable to both Django and FastAPI.
- Generate abstract Python service interfaces and route-registration helpers so
  user implementations can be mounted automatically, similar in role to the
  generated `axum` router pattern.
- Use `minijinja` templates for Python and Python HTTP generation, following the
  existing templated codegen pattern in the repo.
- Align Python HTTP behavior with the semantics already implemented by
  `rust-axum` for HTTP, HTTP security, and HTTP stream, while keeping the Python
  API shape idiomatic.
- Add Python snapshot coverage under `xidlc/tests/` for the core HTTP, security,
  and stream matrix.
- Add Python integration tests and build/test entrypoints modeled after the Go
  example workflow.

## Capabilities

### New Capabilities

- `python-http-target`: defines the `python` and `python-http` target structure,
  runtime boundary, and Python-specific projection of the HTTP, security, and
  stream RFC contracts.

### Modified Capabilities

- `unary-http-mapping`: extend the spec to require Python HTTP support for unary
  request bindings, route normalization, default success status, and metadata
  semantics.
- `http-security-mapping`: extend the spec to require Python HTTP support for
  security inheritance/override, structured requirement metadata, and generated
  auth extraction wiring.
- `http-stream-coverage-tests`: extend coverage requirements so Python HTTP
  participates in snapshot, validation, and end-to-end stream testing.
- `xidlc-security-test-matrix`: extend the security matrix so Python HTTP is
  covered by snapshot and invalid-combination validation tests.
- `xidlc-stream-test-matrix`: extend the stream matrix so Python HTTP is covered
  for supported directions and rejected combinations.

## Impact

- Affected code primarily lives in `xidlc/src/generate/`, `xidlc/src/driver/`,
  shared HTTP/security/stream utilities, and a new Python runtime directory.
- Code generation targets, templates, and test matrices gain a Python dimension.
- Integration coverage adds Python toolchain execution for generated-code
  verification.
