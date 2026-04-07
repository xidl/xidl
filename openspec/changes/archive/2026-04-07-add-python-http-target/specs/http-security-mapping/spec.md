## ADDED Requirements

### Requirement: Python HTTP Effective Security Mapping

The `python-http` target MUST preserve the effective HTTP security requirement
set defined by `docs/rfc/http-security.md`.

#### Scenario: Interface security is inherited in generated Python HTTP code

- **WHEN** a Python HTTP interface declares security annotations and an
  operation declares none
- **THEN** the generated Python HTTP operation metadata and route wiring MUST
  inherit the interface-level requirement set

#### Scenario: Operation security replaces inherited requirements in generated Python HTTP code

- **WHEN** a Python HTTP operation declares one or more security annotations
- **THEN** the generated Python HTTP operation metadata and route wiring MUST
  replace inherited interface-level requirements instead of merging them

#### Scenario: No security clears inherited requirements in generated Python HTTP code

- **WHEN** a Python HTTP operation declares `@no_security`
- **THEN** the generated Python HTTP operation metadata and route wiring MUST
  represent anonymous access for that operation

### Requirement: Python HTTP Security Metadata and Injection

The `python-http` target MUST emit structured security metadata and use it
during generated request handling and client construction.

#### Scenario: Structured security requirements are emitted

- **WHEN** a Python HTTP operation uses basic, bearer, api-key, or oauth2
  annotations
- **THEN** generated Python HTTP code MUST emit structured requirement metadata
  that preserves scheme kind and any declared location, name, or scope details

#### Scenario: Generated route registration applies security extraction

- **WHEN** a secured Python HTTP service implementation is registered
- **THEN** generated route handlers MUST enforce or extract credentials through
  the shared runtime path before invoking the user implementation
