## Why

The current `go-http` target shipped with a minimal happy-path implementation,
but it still trails the published `docs/rfc/http.md`,
`docs/rfc/http-security.md`, and `docs/rfc/http-stream.md` behavior surface.
That leaves the Go generator in an awkward state: the repository documents a
larger contract than the Go target currently proves through code generation and
tests.

## What Changes

- Extend `go-http` generation and runtime support to cover the remaining RFC
  behaviors that are already documented for unary HTTP, HTTP security, and HTTP
  stream mappings.
- Add focused Go HTTP snapshot fixtures for unary bindings, security
  inheritance/override, and stream binding combinations instead of relying on
  the current small fixture set.
- Expand validation coverage so `go-http` asserts the same invalid HTTP
  annotation and stream combinations that other HTTP-capable targets already
  reject.
- Expand `golang/xidlc-examples` integration tests to exercise representative
  unary, security, and stream flows end to end.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `unary-http-mapping`: require `go-http` to implement the published unary HTTP
  request/response semantics it advertises.
- `http-security-mapping`: require `go-http` to honor effective HTTP security
  inheritance, override, and supported scheme mapping semantics.
- `http-spec-coverage-tests`: require Go-target fixture and integration coverage
  for the unary HTTP RFC matrix.
- `http-security-coverage-tests`: require Go-target fixture and integration
  coverage for the HTTP security RFC matrix.
- `http-stream-coverage-tests`: require Go-target fixture and integration
  coverage for the HTTP stream RFC matrix.
- `xidlc-security-test-matrix`: include `go-http` in the security snapshot and
  validation matrix where the target advertises support.
- `xidlc-stream-test-matrix`: include `go-http` in the stream snapshot and
  validation matrix where the target advertises support.

## Impact

- `xidlc/src/generate/go_http/` and shared HTTP validation/plumbing code.
- `golang/xidl-go-http` runtime helpers and generated Go HTTP surfaces.
- `xidlc/tests/` snapshot fixtures and invalid validation coverage.
- `golang/xidlc-examples` end-to-end Go example tests.
