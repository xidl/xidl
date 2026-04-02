## ADDED Requirements

### Requirement: Go HTTP Effective Security Mapping

The `go-http` target MUST preserve the effective HTTP security requirement set
defined by `docs/rfc/http-security.md`.

#### Scenario: Interface security is inherited in generated Go HTTP code

- **WHEN** a Go HTTP interface declares security annotations and an operation
  declares none
- **THEN** the generated Go HTTP operation metadata and handler/client wiring
  MUST inherit the interface-level requirement set

#### Scenario: Operation security replaces inherited requirements in generated Go HTTP code

- **WHEN** a Go HTTP operation declares one or more security annotations
- **THEN** the generated Go HTTP operation metadata and handler/client wiring
  MUST replace inherited interface-level requirements instead of merging them

#### Scenario: No security clears inherited requirements in generated Go HTTP code

- **WHEN** a Go HTTP operation declares `@no_security`
- **THEN** the generated Go HTTP operation metadata and handler/client wiring
  MUST represent anonymous access for that operation

### Requirement: Go HTTP Security Scheme Metadata

The `go-http` target MUST emit structured metadata for the supported HTTP
security schemes instead of collapsing them into target-specific ad hoc logic.

#### Scenario: Basic bearer and API key requirements are emitted structurally

- **WHEN** a Go HTTP operation uses HTTP basic, bearer, or API key annotations
- **THEN** generated Go HTTP code MUST emit structured requirement metadata that
  preserves scheme kind and API key location/name details

#### Scenario: OAuth2 requirements preserve declared scopes

- **WHEN** a Go HTTP operation uses `@oauth2(scopes = [...])`
- **THEN** generated Go HTTP code MUST preserve that requirement and its
  declared scopes in structured metadata so runtime hooks can enforce or apply
  it without reparsing annotations
