## ADDED Requirements

### Requirement: Unary HTTP Missing Value Decoding
The system MUST decode omitted unary HTTP request-side values according to the
unary HTTP RFC default-value rules.

#### Scenario: Missing non-optional query parameter decodes to zero value
- **WHEN** a unary HTTP operation receives a request that omits a non-optional
  query parameter
- **THEN** the generated server-side request model MUST contain the target
  type's RFC-defined zero/default value for that parameter

#### Scenario: Missing non-optional body field decodes to zero value
- **WHEN** a unary HTTP operation receives a JSON body that omits a non-optional
  body field
- **THEN** the generated server-side request model MUST contain the target
  type's RFC-defined zero/default value for that field

#### Scenario: Missing value for type without stable default is rejected
- **WHEN** a unary HTTP operation omits a non-optional request-side value whose
  target type has no stable default in the selected target mapping
- **THEN** request decoding MUST fail rather than inventing a value

### Requirement: Unary HTTP Optional Value Preservation
The system MUST preserve missing unary HTTP request-side values as optional
values only when the target is explicitly annotated with `@optional`.

#### Scenario: Missing optional query parameter decodes to none
- **WHEN** a unary HTTP operation omits a query parameter annotated with
  `@optional`
- **THEN** the generated server-side request model MUST decode that parameter as
  `None`

#### Scenario: Missing optional body field decodes to none
- **WHEN** a unary HTTP operation omits a JSON body field annotated with
  `@optional`
- **THEN** the generated server-side request model MUST decode that field as
  `None`

#### Scenario: Explicit null for non-optional body field is rejected
- **WHEN** a unary HTTP operation receives JSON `null` for a body field that is
  not annotated with `@optional`
- **THEN** request decoding MUST fail with a `400 Bad Request` error

### Requirement: Unary HTTP Media-Type Rejection
The system MUST implement unary HTTP `@Consumes` and `@Produces` as
single-effective-media-type validation, not proactive content negotiation.

#### Scenario: Unsupported request content type is rejected
- **WHEN** a unary HTTP operation expects a request body and the incoming
  `Content-Type` does not match the effective `@Consumes` media type
- **THEN** the runtime MUST reject the request with `415 Unsupported Media Type`

#### Scenario: Unacceptable response media type is rejected
- **WHEN** a unary HTTP operation receives an `Accept` header that excludes the
  effective `@Produces` media type
- **THEN** the runtime MUST reject the request with `406 Not Acceptable`

#### Scenario: No body operation ignores content type
- **WHEN** a unary HTTP operation does not expect a request body
- **THEN** request handling MUST NOT require a matching `Content-Type`

### Requirement: Unary HTTP Deprecation Metadata
The system MUST parse and propagate unary HTTP deprecation annotations and their
normalized time metadata.

#### Scenario: Bare deprecated annotation marks operation deprecated
- **WHEN** an operation or implied attribute operation is annotated with
  `@deprecated`
- **THEN** generated documentation MUST mark that operation as deprecated

#### Scenario: Deprecated shorthand sets since timestamp
- **WHEN** an operation is annotated with `@deprecated("2026-03-13")`
- **THEN** the annotation model MUST treat that value as the normalized `since`
  timestamp for the operation

#### Scenario: Invalid deprecation window is rejected
- **WHEN** an operation is annotated with `@deprecated(since = "...", after = "...")`
  and the normalized `since` timestamp is later than `after`
- **THEN** compilation MUST fail with a validation error

### Requirement: Unary HTTP Default Status and Error Shape
The system MUST apply unary HTTP default success status rules and the RFC error
response shape consistently across generated runtime and OpenAPI output.

#### Scenario: Void unary operation returns no content by default
- **WHEN** a unary HTTP operation has zero response outputs
- **THEN** the runtime and generated OpenAPI MUST use `204 No Content` as the
  default success status

#### Scenario: Unary operation with outputs returns ok by default
- **WHEN** a unary HTTP operation has one or more response outputs
- **THEN** the runtime and generated OpenAPI MUST use `200 OK` as the default
  success status unless another rule explicitly overrides it

#### Scenario: Runtime error body uses numeric http code
- **WHEN** the runtime returns a unary HTTP failure response
- **THEN** the error payload MUST expose `code` as the numeric HTTP status code
  and `msg` as the human-readable summary
