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
