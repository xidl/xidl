## 1. Shared Stream Validation

- [x] 1.1 Audit and consolidate HTTP stream validation so Axum, OpenAPI, and TypeScript use the same normalized stream kind, codec, and method constraints.
- [x] 1.2 Add or update focused compiler tests for supported server-stream SSE and client-stream NDJSON cases.
- [x] 1.3 Add or update validation tests for rejected combinations, including invalid HTTP methods, unsupported codecs, and unsupported bidi projections.

## 2. OpenAPI 3.2.0 Projection

- [x] 2.1 Update the OpenAPI generator to emit `openapi: 3.2.0`.
- [x] 2.2 Change server-stream and client-stream media type rendering to use OpenAPI 3.2.0 `itemSchema`.
- [x] 2.3 Refresh OpenAPI example artifacts and snapshot tests for HTTP stream operations.

## 3. Rust Axum Stream Alignment

- [x] 3.1 Align Rust Axum code generation with the supported HTTP stream matrix for server-stream SSE and client-stream NDJSON operations.
- [x] 3.2 Update `xidl-rust-axum` stream helpers or generated client code where needed so headers and body framing match the supported contract.
- [x] 3.3 Add or refresh Rust Axum example and integration coverage for stream request and response handling.

## 4. TypeScript Stream Alignment

- [x] 4.1 Align TypeScript generator output for server-stream methods with the supported SSE async-iterable client shape.
- [x] 4.2 Align TypeScript generator output for client-stream methods with NDJSON async-iterable request bodies and final unary responses.
- [x] 4.3 Add or refresh TypeScript generator tests for supported stream methods and rejected unsupported shapes.

## 5. End-to-End Verification

- [x] 5.1 Regenerate the canonical HTTP stream example outputs across Axum, OpenAPI, and TypeScript targets.
- [x] 5.2 Run focused generator, snapshot, and example tests covering the updated HTTP stream contract.
- [x] 5.3 Review generated diffs for compatibility impact and document any intentional behavior changes in the change notes or follow-up issues.
