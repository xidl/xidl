## ADDED Requirements

### Requirement: HTTP Security Generator Coverage Matrix

The `xidlc` test suite MUST cover the published HTTP security mapping behavior
through focused positive fixtures for inheritance, override, anonymous access,
and supported scheme projection.

#### Scenario: Effective security resolution is covered

- **WHEN** interface-level and operation-level security annotations influence an
  operation's effective requirement set
- **THEN** `xidlc/tests` MUST include fixtures that cover inheritance,
  replacement, and explicit clearing via `@no-security`

#### Scenario: Supported security schemes are covered

- **WHEN** the generators support HTTP basic, bearer, api-key, or oauth2
  mappings
- **THEN** the test suite MUST include fixtures that verify those schemes appear
  correctly in the relevant generated outputs

### Requirement: HTTP Security Invalid Coverage

The `xidlc` test suite MUST assert invalid HTTP security combinations and
parameter errors rather than relying only on happy-path coverage.

#### Scenario: Duplicate and conflicting annotations are asserted

- **WHEN** security annotations are duplicated or `@no-security` conflicts with
  authenticated access
- **THEN** the test suite MUST include targeted assertions on the resulting
  validation errors

#### Scenario: Invalid security parameterization is asserted

- **WHEN** an HTTP security annotation is missing required parameters or uses
  unsupported parameter values
- **THEN** the test suite MUST include targeted assertions on the resulting
  validation errors

### Requirement: HTTP Security Example Integration Coverage

The `xidlc-examples` package MUST include integration checks for generated
security-aware HTTP or document outputs where runtime interaction adds coverage
value beyond snapshots.

#### Scenario: Example tests validate representative security-backed flows

- **WHEN** example services expose security-relevant generated behavior
- **THEN** `xidlc-examples/tests` MUST include integration assertions that
  protect those flows from regression

#### Scenario: Example tests complement generator-level security coverage

- **WHEN** a security behavior is already represented in `xidlc/tests`
- **THEN** `xidlc-examples/tests` MUST focus on end-to-end generated-code
  behavior instead of duplicating the same fixture-level assertion verbatim

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
