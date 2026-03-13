## 1. Parser and Validation

- [x] 1.1 Add parser and HIR support for unary HTTP deprecation and HTTP security annotations, including hyphenated names.
- [x] 1.2 Implement validation for invalid deprecation windows and invalid security annotation combinations.
- [x] 1.3 Add parser- and HIR-level tests covering `@deprecated`, `@http-basic`, `@http-bearer`, `@api-key`, `@oauth2`, and `@no-security`.

## 2. Unary HTTP Mapping

- [x] 2.1 Update unary HTTP code generation metadata so request-side values follow RFC default-value and `@optional` semantics.
- [x] 2.2 Update `xidl-rust-axum` request decoding and error handling to enforce RFC media-type rejection and non-optional `null` rejection.
- [x] 2.3 Align unary HTTP success status and error body behavior with the RFC defaults in generated server/client paths.

## 3. OpenAPI and Documentation Projection

- [x] 3.1 Update OpenAPI generation to emit unary HTTP deprecation metadata and RFC-aligned default status behavior.
- [x] 3.2 Update OpenAPI generation to emit security schemes and operation security requirements for unary HTTP security annotations.
- [x] 3.3 Add or refresh snapshot tests for unary HTTP and OpenAPI outputs that exercise the new deprecation and security cases.

## 4. Examples and Verification

- [x] 4.1 Add or update example IDLs to cover omitted-value defaults, `@optional`, and unary HTTP security annotations.
- [x] 4.2 Run focused generator, snapshot, and axum runtime tests for the updated unary HTTP profile.
- [x] 4.3 Review generated output diffs for compatibility impact and document any intentional behavior changes in the change notes.
