## ADDED Requirements

### Requirement: Go HTTP Unary RFC Projection

The `go-http` target MUST implement the unary HTTP mapping behaviors published
in `docs/rfc/http.md` for the subset of features it advertises.

#### Scenario: Go HTTP request bindings follow RFC source resolution

- **WHEN** a Go HTTP unary operation uses path, query, header, cookie, and body
  inputs or relies on default source resolution
- **THEN** generated Go handlers and clients MUST bind those values according to
  the effective unary HTTP RFC rules

#### Scenario: Go HTTP unary routes preserve normalization and template semantics

- **WHEN** a Go HTTP unary operation uses explicit paths, catch-all bindings, or
  query-template variables
- **THEN** generated Go HTTP code MUST preserve the RFC route normalization and
  binding semantics for those routes

### Requirement: Go HTTP Unary Response and Metadata Semantics

The `go-http` target MUST preserve unary HTTP response and metadata semantics
that are visible to generated code consumers.

#### Scenario: Go HTTP unary responses preserve status and body shape defaults

- **WHEN** a Go HTTP unary operation has no response outputs or one or more
  response outputs
- **THEN** generated Go server and client code MUST preserve the RFC default
  `204` and `200` success semantics together with the expected response-body
  shape

#### Scenario: Go HTTP unary metadata preserves deprecation information

- **WHEN** a Go HTTP unary operation is annotated with RFC deprecation metadata
- **THEN** generated Go output MUST preserve the normalized deprecated state in
  emitted code or metadata helpers rather than discarding it silently
