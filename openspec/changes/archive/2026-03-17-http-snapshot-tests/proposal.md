## Why

We need a deterministic HTTP snapshot test framework for xidlc-examples to validate headers and bodies in real server responses. This will make HTTP behavior changes visible and reviewable via snapshots, with configurable runtime variables.

## What Changes

- Add an HTTP snapshot test definition format (Hurl-like) with minijinja variable injection.
- Implement a snapshot runner that executes requests and captures full request/response transcripts.
- Generate one snapshot file per definition file, similar to existing codegen snapshot workflows.
- Add initial HTTP snapshots for `xidlc-examples/api/http/http_server.idl`.

## Capabilities

### New Capabilities
- `http-snapshot-tests`: Define, run, and snapshot HTTP test flows with header/body capture and variable injection.

### Modified Capabilities
- (none)

## Impact

- New test harness and snapshot artifacts in `xidlc-examples`.
- New test definitions and snapshots for HTTP examples.
- Potential updates to test runner utilities and documentation for snapshot usage.
