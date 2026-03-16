## 1. Version Selection Logic

- [x] 1.1 Locate OpenAPI version emission in `xidlc/src/generate/openapi` and
      document current default behavior
- [x] 1.2 Introduce a version selection helper that returns 3.1 or 3.2 based on
      feature usage
- [x] 1.3 Define and wire detection for 3.2-only features (e.g., stream
      `itemSchema`) used by the generator

## 2. Generator Updates

- [x] 2.1 Update document writer to use the selected version for the top-level
      `openapi` field
- [x] 2.2 Ensure server/client stream projections trigger 3.2 output when they
      require 3.2-only features

## 3. Tests and Fixtures

- [x] 3.1 Update or add tests asserting default 3.1 output when no 3.2-only
      features are present
- [x] 3.2 Update golden OpenAPI fixtures/snapshots that currently expect 3.2
- [x] 3.3 Add a test case that verifies 3.2 output is selected when 3.2-only
      features are used

## 4. Documentation and Validation

- [x] 4.1 Update any OpenAPI version validation messages to mention the
      conditional 3.1/3.2 behavior
- [x] 4.2 Document the new default behavior for OpenAPI versioning (if a
      user-facing doc exists)
