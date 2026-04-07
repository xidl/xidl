## ADDED Requirements

### Requirement: Python HTTP Unary RFC Projection

The `python-http` target MUST implement the unary HTTP mapping behaviors
published in `docs/rfc/http.md` for the subset of features it advertises.

#### Scenario: Python HTTP request bindings follow RFC source resolution

- **WHEN** a Python HTTP unary operation uses path, query, header, cookie, or
  body inputs, or relies on default source resolution
- **THEN** generated Python handlers and clients MUST bind those values
  according to the effective unary HTTP RFC rules

#### Scenario: Python HTTP unary routes preserve normalization and templates

- **WHEN** a Python HTTP unary operation uses explicit paths, catch-all
  bindings, or query-template variables
- **THEN** generated Python HTTP code MUST preserve RFC route normalization and
  binding semantics for those routes

### Requirement: Python HTTP Unary Service Registration

The `python-http` target MUST expose unary operations through generated abstract
service methods and automatic route registration helpers.

#### Scenario: Unary operation is represented as generated abstract method

- **WHEN** a Python HTTP unary interface is generated
- **THEN** each unary operation MUST appear on the generated abstract service
  interface as an implementation method contract

#### Scenario: Unary operation can be mounted without handwritten route glue

- **WHEN** a concrete Python HTTP service implementation is registered
- **THEN** generated route helpers MUST mount unary routes automatically without
  requiring handwritten per-operation binding code
