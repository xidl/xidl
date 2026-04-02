## ADDED Requirements

### Requirement: Go HTTP Stream Coverage Matrix

The `xidlc` and Go example test suites MUST cover the HTTP stream behaviors
advertised by the `go-http` target.

#### Scenario: Go HTTP stream snapshot fixtures cover supported directions and bindings

- **WHEN** the repository validates Go HTTP stream generation
- **THEN** `xidlc/tests/golang-http/` MUST include focused fixtures for the
  supported server-stream and client-stream directions together with request
  binding and final unary response semantics

#### Scenario: Go HTTP stream examples protect runtime framing behavior

- **WHEN** `golang/xidlc-examples` validates generated Go HTTP stream code
- **THEN** its tests MUST execute representative SSE and NDJSON flows that prove
  auth propagation, stream item delivery, and final unary response handling

### Requirement: Go HTTP Stream Validation Coverage

The `xidlc` test suite MUST assert unsupported HTTP stream combinations for the
`go-http` target explicitly.

#### Scenario: Go HTTP invalid stream combinations are asserted explicitly

- **WHEN** a Go HTTP stream definition uses an unsupported codec, invalid HTTP
  method, mutually exclusive annotations, or an unsupported client-stream shape
- **THEN** `xidlc/tests/http_validation.rs` MUST include targeted Go HTTP
  assertions on the resulting validation error
