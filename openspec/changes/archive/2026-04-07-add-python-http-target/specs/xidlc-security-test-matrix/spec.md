## ADDED Requirements

### Requirement: Python HTTP Participates in Security Matrix Coverage

The `xidlc` security snapshot and validation matrix MUST include `python-http`
for the HTTP security behaviors that target advertises.

#### Scenario: Python HTTP security fixtures are present in the matrix

- **WHEN** `python-http` advertises an HTTP security behavior
- **THEN** `xidlc/tests/python-http/` MUST include a focused fixture or
  assertion that represents that behavior in the security matrix

#### Scenario: Python HTTP security validation stays aligned with shared rules

- **WHEN** shared HTTP security validation rejects a definition that also
  applies to `python-http`
- **THEN** the matrix tests MUST assert the Python HTTP failure path explicitly
