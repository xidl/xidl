## ADDED Requirements

### Requirement: Go HTTP Unary Coverage Matrix

The `xidlc` and Go example test suites MUST cover the unary HTTP RFC behaviors
advertised by the `go-http` target with focused fixtures and end-to-end tests.

#### Scenario: Go HTTP unary snapshot fixtures cover binding and response categories

- **WHEN** the repository validates Go HTTP unary code generation
- **THEN** `xidlc/tests/golang-http/` MUST include focused fixtures for path,
  query, header, cookie, defaults, media types, response-shape, and
  route-template behavior

#### Scenario: Go HTTP unary examples protect runtime semantics

- **WHEN** `golang/xidlc-examples` validates generated unary Go HTTP code
- **THEN** its tests MUST execute representative requests that prove request
  binding, status handling, and response decoding behavior through real
  client/server interaction

### Requirement: Go HTTP Unary Validation Coverage

The `xidlc` test suite MUST assert the unary HTTP validation failures that the
`go-http` target is expected to reject.

#### Scenario: Go HTTP invalid unary bindings are asserted explicitly

- **WHEN** a unary HTTP definition violates a binding or route-template rule
  that `go-http` enforces
- **THEN** `xidlc/tests/http_validation.rs` MUST include a Go HTTP assertion on
  the resulting validation error
