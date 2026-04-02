## ADDED Requirements

### Requirement: Go HTTP Participates in Stream Matrix Coverage

The `xidlc` stream snapshot and validation matrix MUST include `go-http` for
each HTTP stream direction and constraint that target advertises.

#### Scenario: Go HTTP supported stream directions are represented in the matrix

- **WHEN** `go-http` advertises support for server-stream or client-stream
- **THEN** `xidlc/tests/golang-http/` MUST include dedicated fixtures for those
  supported directions

#### Scenario: Go HTTP unsupported stream combinations are represented in the matrix

- **WHEN** `go-http` rejects an HTTP stream codec, method, or shape
- **THEN** the stream matrix tests MUST assert the Go HTTP rejection path
  explicitly
