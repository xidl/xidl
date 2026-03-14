## 1. Audit Coverage Against The Three Specs

- [x] 1.1 Map the existing `xidlc/tests` fixtures and validation tests to the `http`, `http-security`, and `http-stream` spec requirements, and list the uncovered matrix cells by target.
- [x] 1.2 Map the existing `xidlc-examples/tests` integration tests to the same three specs, and identify which end-to-end behaviors are still missing.

## 2. Expand `xidlc/tests` Unary HTTP Coverage

- [x] 2.1 Add missing positive unary HTTP fixtures and snapshots so `axum`, `openapi`, and `ts` cover their supported binding, default, route-template, and response-shape behaviors explicitly.
- [x] 2.2 Add focused invalid unary HTTP tests for unsupported or malformed binding combinations that should fail generation or validation.

## 3. Expand `xidlc/tests` Security And Stream Coverage

- [x] 3.1 Add missing positive HTTP security fixtures and snapshots for inheritance, override, anonymous access, supported schemes, and security interactions with streamed operations.
- [x] 3.2 Add missing negative HTTP security tests for duplicate annotations, conflicting combinations, and invalid security parameters.
- [x] 3.3 Add missing positive HTTP stream fixtures and snapshots for supported server-stream, client-stream, and bidi-stream behaviors, including binding and final-response semantics.
- [x] 3.4 Add missing negative HTTP stream tests for unsupported methods, codecs, mutually exclusive annotations, unsupported directions, and target-specific stream constraints.

## 4. Expand `xidlc-examples` Integration Coverage

- [x] 4.1 Extend unary HTTP example integration tests so generated clients and servers exercise representative spec-backed binding and response behaviors end to end.
- [x] 4.2 Extend HTTP security and HTTP stream example integration tests so generated code proves representative security-sensitive and streamed workflows over real client/server interactions.

## 5. Verify And Refresh Outputs

- [x] 5.1 Refresh or add snapshots and generated example artifacts affected by the new fixtures and example IDL inputs.
- [x] 5.2 Run the relevant `xidlc` and `xidlc-examples` test suites to verify the expanded coverage passes cleanly.
