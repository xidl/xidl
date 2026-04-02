## ADDED Requirements

### Requirement: Go HTTP Participates in Security Matrix Coverage

The `xidlc` security snapshot and validation matrix MUST include `go-http` for
the HTTP security behaviors that target advertises.

#### Scenario: Go HTTP security fixtures are present in the matrix

- **WHEN** `go-http` advertises support for an HTTP security behavior
- **THEN** `xidlc/tests/golang-http/` MUST include a focused fixture or
  assertion that represents that behavior in the security matrix

#### Scenario: Go HTTP security validation stays aligned with shared rules

- **WHEN** shared HTTP security validation rejects a definition that also
  applies to `go-http`
- **THEN** the matrix tests MUST assert the Go HTTP failure path explicitly
