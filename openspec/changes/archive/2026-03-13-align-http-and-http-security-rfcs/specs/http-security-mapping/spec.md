## ADDED Requirements

### Requirement: HTTP Security Annotation Parsing
The system MUST accept and represent unary HTTP security annotations defined by
the HTTP security RFC.

#### Scenario: Http basic annotation is parsed
- **WHEN** an interface or operation is annotated with `@http-basic`
- **THEN** the parser and HIR MUST represent that annotation as a unary HTTP
  security requirement

#### Scenario: Http bearer annotation is parsed
- **WHEN** an interface or operation is annotated with `@http-bearer`
- **THEN** the parser and HIR MUST represent that annotation as a unary HTTP
  security requirement

#### Scenario: No security annotation is parsed
- **WHEN** an operation is annotated with `@no-security`
- **THEN** the parser and HIR MUST represent that annotation as an explicit
  request for anonymous access

### Requirement: HTTP Security Inheritance and Override
The system MUST apply unary HTTP security annotations using the RFC inheritance
and override model.

#### Scenario: Interface security is inherited by operation
- **WHEN** an interface declares unary HTTP security annotations and an
  operation declares none
- **THEN** the operation's effective security requirements MUST inherit the
  interface-level requirement set

#### Scenario: Operation security replaces inherited security
- **WHEN** an operation declares one or more unary HTTP security annotations
- **THEN** the operation's effective security requirements MUST replace, not
  merge with, inherited interface-level requirements

#### Scenario: No security clears inherited security
- **WHEN** an operation is annotated with `@no-security`
- **THEN** the operation's effective security requirements MUST be anonymous
  access and inherited interface-level requirements MUST be ignored

### Requirement: HTTP Security Validation
The system MUST reject invalid unary HTTP security annotation combinations.

#### Scenario: No security cannot be combined with other security annotations
- **WHEN** an operation declares `@no-security` together with any other unary
  HTTP security annotation
- **THEN** compilation MUST fail with a validation error

#### Scenario: Duplicate http basic annotation is rejected
- **WHEN** the same interface or operation declares `@http-basic` more than once
- **THEN** compilation MUST fail with a validation error

#### Scenario: Duplicate http bearer annotation is rejected
- **WHEN** the same interface or operation declares `@http-bearer` more than
  once
- **THEN** compilation MUST fail with a validation error

### Requirement: HTTP Security OpenAPI Mapping
The system MUST propagate unary HTTP security requirements into generated
OpenAPI output.

#### Scenario: Http basic maps to security scheme
- **WHEN** a unary HTTP interface or operation uses `@http-basic`
- **THEN** generated OpenAPI MUST include a corresponding HTTP Basic security
  scheme and reference it from the affected operation

#### Scenario: Http bearer maps to security scheme
- **WHEN** a unary HTTP interface or operation uses `@http-bearer`
- **THEN** generated OpenAPI MUST include a corresponding HTTP Bearer security
  scheme and reference it from the affected operation

#### Scenario: No security maps to empty operation security
- **WHEN** an operation uses `@no-security`
- **THEN** generated OpenAPI MUST mark that operation with an explicit empty
  security requirement set
