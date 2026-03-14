## Why

The repository now has meaningful coverage for HTTP, HTTP security, and HTTP
stream generation, but the coverage is still uneven across targets and does not
yet guarantee that the published specs stay protected from regressions. The
missing cases are concentrated in unsupported combinations, cross-target gaps,
and `xidlc-examples` integration paths, which makes it easy for behavior to
drift without a focused test matrix.

## What Changes

- Expand `xidlc/tests` so the `http`, `http-security`, and `http-stream`
  specs each have an explicit positive and negative coverage matrix across the
  relevant generator targets.
- Add missing targeted validation tests for invalid annotation combinations,
  unsupported transport/method/shape combinations, and target-specific
  rejection paths.
- Add or extend `xidlc-examples` integration tests so generated HTTP and stream
  examples exercise the same spec-backed behavior through real client/server
  flows.
- Document the intended coverage boundaries in OpenSpec artifacts so future
  work can tell whether a new behavior needs fixture coverage, integration
  coverage, or both.

## Capabilities

### New Capabilities
- `http-spec-coverage-tests`: Defines the required test coverage for unary HTTP
  mapping behavior in `xidlc/tests` and `xidlc-examples`.
- `http-security-coverage-tests`: Defines the required coverage matrix for HTTP
  security inheritance, overrides, invalid combinations, and generated target
  output.
- `http-stream-coverage-tests`: Defines the required coverage matrix for HTTP
  stream projections, unsupported combinations, and end-to-end stream example
  behavior.

### Modified Capabilities
- None.

## Impact

- Affected code: `xidlc/tests`, `xidlc-examples/tests`, example IDL inputs, and
  any helper code needed to assert target-specific validation errors.
- Affected APIs: generated snapshot outputs and example integration behavior
  for HTTP, HTTP security, and HTTP stream targets.
- Risk areas: adding overlapping fixtures that obscure intent, missing
  target-specific negative paths, and leaving `xidlc-examples` integration
  coverage behind the generator-level test matrix.
