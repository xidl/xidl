## ADDED Requirements

### Requirement: Python HTTP Participates in Stream Matrix Coverage

The `xidlc` stream snapshot and validation matrix MUST include `python-http` for
each HTTP stream direction and constraint that target advertises.

#### Scenario: Python HTTP supported stream directions are represented in the matrix

- **WHEN** `python-http` advertises support for server-stream or client-stream
- **THEN** `xidlc/tests/python-http/` MUST include dedicated fixtures for those
  supported directions

#### Scenario: Python HTTP unsupported stream combinations are represented in the matrix

- **WHEN** `python-http` rejects an HTTP stream codec, method, or shape
- **THEN** the stream matrix tests MUST assert the Python HTTP rejection path
  explicitly
