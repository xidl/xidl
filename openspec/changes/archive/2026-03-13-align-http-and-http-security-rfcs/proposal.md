## Why

The unary HTTP mapping and the new HTTP security draft now define behavior that
the compiler, OpenAPI generator, and `xidl-rust-axum` runtime do not yet
implement consistently. This change aligns the implementation with the RFCs so
the documented contract, generated artifacts, and runtime behavior stop
drifting apart.

## What Changes

- Implement unary HTTP RFC behavior for missing values, `@optional`,
  `@Consumes`/`@Produces` validation, deprecation metadata, and response status
  defaults across parsing, code generation, and runtime.
- Add support for unary HTTP security annotations, including `@http-basic`,
  `@http-bearer`, `@api-key`, `@oauth2`, and `@no-security`, so these
  annotations can be parsed, represented, and emitted into generated OpenAPI.
- Update generated OpenAPI output so HTTP and HTTP security semantics are
  documented in a way that matches the RFCs and the runtime profile.
- Add validation and tests for the new annotation forms and the revised unary
  HTTP decoding rules.

## Capabilities

### New Capabilities
- `unary-http-mapping`: Defines the normative unary HTTP contract for default
  value decoding, optional value preservation, media-type rejection,
  deprecation metadata, and default success/error status handling.
- `http-security-mapping`: Defines unary HTTP security annotations and their
  inheritance, override, and OpenAPI mapping behavior.

### Modified Capabilities
- None.

## Impact

- Affected code: parser / HIR annotation handling, `xidlc` HTTP and OpenAPI
  generators, `xidl-rust-axum` runtime request decoding and response metadata,
  and related snapshot / integration tests.
- Affected APIs: generated Rust axum server/client behavior, generated OpenAPI
  documents, and accepted annotation syntax in IDL.
- Risk areas: default value semantics for omitted fields, interaction between
  runtime behavior and OpenAPI schema generation, and parser support for
  hyphenated annotation names.

## Change Notes

- Unary HTTP parsing now accepts hyphenated annotations such as
  `@http-basic`, `@http-bearer`, `@api-key`, and `@no-security`.
- Generated unary HTTP request models now default missing
  query/header/cookie values to Rust `Default::default()` unless `@optional`
  is present.
- Generated unary HTTP body helper structs now default missing non-optional
  JSON members and reject explicit `null` for those members.
- Generated unary HTTP clients now send `Accept` for the effective response
  media type.
- OpenAPI output now emits deprecation flags and unary HTTP security schemes /
  requirements for the RFC annotations.
