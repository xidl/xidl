## Why

`xidlc/tests` already covers a few happy-path stream and security cases, but it
does not yet exercise the broader compatibility matrix or enough invalid
combinations. As the stream and HTTP security contracts grow, the repository
needs denser test fixtures so regressions show up in snapshots and validation
tests instead of slipping into generated outputs unnoticed.

## What Changes

- Add more `xidlc/tests/{axum,openapi,ts}` IDL fixtures for stream behavior,
  including valid permutations and unsupported combinations that must fail.
- Add more HTTP security fixtures covering inheritance, overrides, duplicate or
  conflicting annotations, and interactions with stream operations.
- Extend snapshot and targeted test coverage so new fixtures are exercised in
  the existing `xidlc` test workflow.
- Document the intended coverage matrix so future stream and security work can
  add tests systematically instead of ad hoc.

## Capabilities

### New Capabilities
- `xidlc-stream-test-matrix`: Defines the required breadth of stream-oriented
  test fixtures across supported `xidlc` generator targets.
- `xidlc-security-test-matrix`: Defines the required breadth of HTTP security
  test fixtures, including valid and invalid annotation combinations.

### Modified Capabilities
- None.

## Impact

- Affected code: `xidlc/tests` IDL fixtures, snapshot coverage, and targeted
  generator validation tests.
- Affected APIs: generated snapshot outputs for `axum`, `openapi`, and
  `typescript`, plus parser/generator error expectations for invalid inputs.
- Risk areas: adding too many overlapping fixtures without a clear matrix,
  creating target-specific cases that duplicate each other unnecessarily, and
  growing snapshots without improving failure localization.
