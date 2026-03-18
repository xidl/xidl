## ADDED Requirements

### Requirement: Bearer auth runtime type

The system MUST provide a Bearer auth type and extractor in the axum runtime
module for generated servers.

#### Scenario: Bearer token is extracted

- **WHEN** the request includes an `Authorization: Bearer <token>` header
- **THEN** the extractor MUST return a Bearer auth value whose token equals
  `<token>`

#### Scenario: Empty bearer token yields default value

- **WHEN** the request includes an `Authorization: Bearer` header with an empty
  token
- **THEN** the extractor MUST return a Bearer auth value whose token equals the
  default string value

### Requirement: Auth field on request payload for bearer endpoints

The system MUST add `xidl_auth` to the request payload for Bearer-protected
operations, carried inside `xidl_rust_axum::Request<T>`.

#### Scenario: Bearer auth request shape is generated

- **WHEN** an operation is annotated with `@http-bearer`
- **THEN** the generated handler input type MUST be `xidl_rust_axum::Request<T>`
  where `T` includes `xidl_auth: BearerAuth`
