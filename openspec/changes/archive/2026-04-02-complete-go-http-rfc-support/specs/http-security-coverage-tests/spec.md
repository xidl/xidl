## ADDED Requirements

### Requirement: Go HTTP Security Coverage Matrix

The `xidlc` and Go example test suites MUST cover the HTTP security behaviors
advertised by the `go-http` target.

#### Scenario: Go HTTP security fixtures cover effective requirement resolution

- **WHEN** the repository validates Go HTTP security code generation
- **THEN** `xidlc/tests/golang-http/` MUST include focused fixtures for
  interface inheritance, operation replacement, anonymous override, and the
  supported security schemes

#### Scenario: Go HTTP security examples protect auth-aware runtime flows

- **WHEN** `golang/xidlc-examples` validates generated Go HTTP security code
- **THEN** its tests MUST execute representative requests that prove client auth
  application and server-side auth extraction for supported schemes

### Requirement: Go HTTP Security Validation Coverage

The `xidlc` test suite MUST assert invalid HTTP security combinations for the
`go-http` target when the shared HTTP validation rules apply to it.

#### Scenario: Go HTTP invalid security annotations are asserted explicitly

- **WHEN** duplicated, conflicting, or malformed HTTP security annotations are
  rejected for Go HTTP generation
- **THEN** `xidlc/tests/http_validation.rs` MUST include targeted Go HTTP
  assertions for those failures
