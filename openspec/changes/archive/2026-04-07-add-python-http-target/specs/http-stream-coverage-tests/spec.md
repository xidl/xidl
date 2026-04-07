## ADDED Requirements

### Requirement: Python HTTP Stream Coverage Matrix

The `xidlc` and Python example test suites MUST cover the HTTP stream behaviors
advertised by the `python-http` target.

#### Scenario: Python HTTP stream snapshot fixtures cover supported directions and bindings

- **WHEN** the repository validates Python HTTP stream generation
- **THEN** `xidlc/tests/python-http/` MUST include focused fixtures for the
  supported server-stream and client-stream directions together with request
  binding and final unary response semantics

#### Scenario: Python HTTP stream examples protect runtime framing behavior

- **WHEN** Python integration tests validate generated Python HTTP stream code
- **THEN** they MUST execute representative SSE and NDJSON flows that prove auth
  propagation, stream item delivery, and final unary response handling

### Requirement: Python HTTP Stream Adapter Coverage

Python integration coverage MUST prove that generated stream routing remains
usable through the supported framework adapters.

#### Scenario: Django adapter exercises a generated stream route

- **WHEN** Python integration tests run the Django adapter path for a generated
  stream service
- **THEN** the test suite MUST verify that the generated route registration and
  runtime framing still behave according to the advertised stream contract

#### Scenario: FastAPI adapter exercises a generated stream route

- **WHEN** Python integration tests run the FastAPI adapter path for a generated
  stream service
- **THEN** the test suite MUST verify that the generated route registration and
  runtime framing still behave according to the advertised stream contract
