## Why

`rust-axum` currently provides the repository's most complete implementation of
the unary HTTP, HTTP security, and HTTP stream RFCs. That leaves XIDL without a
Go-native target for teams that want the same contract-first workflow in Go:

- plain Go data and type projection
- generated HTTP client/server surfaces aligned with `docs/rfc/http.md`
- security handling aligned with `docs/rfc/http-security.md`
- streaming behavior aligned with `docs/rfc/http-stream.md`

We need a Go story that mirrors the existing Rust split:

- a language target for plain generated models (`go`)
- an HTTP-oriented target with runtime support (`go-http`)

This must come with examples and tests, not just generator output, so the Go
projection becomes a first-class maintained target instead of an experimental
dump of files.

## What Changes

- Add a `go` generator target for Go types and non-transport interface
  projections.
- Add a `go-http` generator target for HTTP APIs with generated clients, handler
  registration, request/response shaping, security, and stream support.
- Add a Go HTTP runtime crate/package structure, with a framework-neutral
  standard-library base and room for optional adapters later.
- Add a new `xidlc-examples-go` workspace member that mirrors `xidlc-examples`
  for Go.
- Extend `xidlc/tests` with Go snapshot coverage and add end-to-end Go example
  tests.

## Capabilities

### New Capabilities

- `go-target`: Generate Go types and interface-facing code from XIDL.
- `go-http-target`: Generate Go HTTP clients and servers aligned with the HTTP,
  HTTP security, and HTTP stream RFCs.
- `go-example-suite`: Validate generated Go output through examples and
  integration tests.

### Modified Capabilities

- `xidlc-stream-test-matrix`: Add Go HTTP stream coverage.
- `xidlc-security-test-matrix`: Add Go HTTP security coverage.
- `http-spec-coverage-tests`: Add Go-target snapshot coverage where applicable.

## Impact

- New generator modules under `xidlc/src/generate/`.
- New Go runtime package(s) in the workspace.
- New Go examples and tests in a separate example workspace member.
- New snapshot and integration coverage in `xidlc/tests`.
