## ADDED Requirements

### Requirement: HTTP Bearer maps to axum server auth

The system MUST propagate unary HTTP Bearer security requirements into generated
axum server handlers.

#### Scenario: Bearer auth is mapped into server handler

- **WHEN** a unary HTTP operation is annotated with `@http-bearer`
- **THEN** generated axum server code MUST extract bearer auth and supply it by
  adding `xidl_auth` to the handler request payload type

#### Scenario: No security does not add auth wrapper

- **WHEN** a unary HTTP operation is annotated with `@no-security`
- **THEN** generated axum server code MUST NOT wrap the handler input type with
  auth data
