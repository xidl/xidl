## Why

The OpenAPI generator currently defaults to 3.2, but many downstream tools do
not support 3.2 yet. Defaulting to 3.1 improves interoperability while still
allowing 3.2 when required by specific features.

## What Changes

- Default the OpenAPI document version emitted by `xidlc/src/generate/openapi`
  to 3.1.
- Emit 3.2 only when the generated document requires 3.2-only features.
- Update version-related validation, tests, and documentation to reflect the new
  default behavior.

## Capabilities

### New Capabilities

- (none)

### Modified Capabilities

- `http-stream-openapi`: Adjust the OpenAPI version requirement to default to
  3.1 and upgrade to 3.2 only when needed for required features.

## Impact

- OpenAPI generator logic in `xidlc/src/generate/openapi`.
- OpenSpec requirements for OpenAPI versioning.
- Golden test fixtures or snapshots for generated OpenAPI output.
