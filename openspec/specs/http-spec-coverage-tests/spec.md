## ADDED Requirements

### Requirement: Unary HTTP Generator Coverage Matrix

The `xidlc` test suite MUST cover the published unary HTTP mapping behavior for
each relevant generator target using focused fixtures instead of relying on a
small number of catch-all files.

#### Scenario: Unary HTTP binding categories are covered

- **WHEN** the repository validates unary HTTP mapping behavior
- **THEN** `xidlc/tests` MUST include fixture coverage for path, query, header,
  cookie, defaults, response-shape, and route-template behavior across the
  supported generator targets

#### Scenario: Unary HTTP target gaps are filled deliberately

- **WHEN** one target has less unary HTTP coverage than the others
- **THEN** the test suite MUST add target-specific fixtures or assertions until
  the target's supported unary HTTP behaviors are represented explicitly

### Requirement: Unary HTTP Validation Coverage

The `xidlc` test suite MUST assert failures for invalid unary HTTP annotation
and binding combinations that the generators are expected to reject.

#### Scenario: Invalid unary HTTP bindings are asserted

- **WHEN** a unary HTTP definition violates target binding rules
- **THEN** the test suite MUST include a targeted assertion on the resulting
  validation error or generator failure path

#### Scenario: Unary HTTP tests remain separate from stream-only coverage

- **WHEN** a failure concerns unary HTTP mapping rather than stream semantics
- **THEN** the test suite MUST verify it through unary HTTP fixtures instead of
  only through stream-oriented cases

### Requirement: Unary HTTP Example Integration Coverage

The `xidlc-examples` package MUST include integration tests that exercise
generated unary HTTP clients and servers for representative spec-backed flows.

#### Scenario: Example tests cover representative unary HTTP bindings

- **WHEN** `xidlc-examples` validates generated unary HTTP code
- **THEN** its integration tests MUST execute requests that prove representative
  binding and response-shape behavior through real client/server interaction

#### Scenario: Example tests protect generated unary HTTP regressions

- **WHEN** generated unary HTTP code changes in a way that breaks request
  binding or response handling
- **THEN** `xidlc-examples/tests` MUST fail without relying on snapshot output
  alone

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
