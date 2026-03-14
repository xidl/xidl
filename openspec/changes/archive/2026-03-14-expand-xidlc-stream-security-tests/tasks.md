## 1. Audit The Existing Coverage Matrix

- [x] 1.1 Inventory the current stream fixtures under `xidlc/tests/{axum,openapi,ts}` and map which supported server-stream and client-stream cases are still missing.
- [x] 1.2 Inventory the current HTTP security fixtures and identify missing inheritance, override, anonymous-access, duplicate, and stream-interaction cases.

## 2. Add Stream Coverage

- [x] 2.1 Add new positive stream `.idl` fixtures under the relevant target test folders for the supported stream matrix, including request binding interactions and final unary responses.
- [x] 2.2 Add focused invalid stream fixtures and targeted test assertions for unsupported codecs, invalid HTTP methods, and unsupported directionality such as bidi-stream where the target must reject generation.

## 3. Add Security Coverage

- [x] 3.1 Add new positive HTTP security `.idl` fixtures covering inheritance, operation-level replacement, explicit `@no-security`, multiple supported schemes, and stream-operation security interactions.
- [x] 3.2 Add focused invalid security fixtures and targeted test assertions for duplicate annotations and conflicting authenticated plus anonymous combinations.

## 4. Verify The Expanded Suite

- [x] 4.1 Refresh or add the expected snapshots for all new positive fixtures.
- [x] 4.2 Run the relevant `xidlc` tests to verify both the snapshot coverage and the new negative validation assertions pass.
