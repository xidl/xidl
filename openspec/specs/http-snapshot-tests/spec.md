# http-snapshot-tests Specification

## Purpose
TBD - created by archiving change http-snapshot-tests. Update Purpose after archive.
## Requirements
### Requirement: Snapshot definition format
The system SHALL accept HTTP snapshot definition files in a Hurl-like syntax that can declare multiple HTTP tests per file, including method, path, headers, and body.

#### Scenario: Multiple requests in one definition file
- **WHEN** a definition file declares more than one HTTP test
- **THEN** the runner executes each test in order and records a single combined snapshot

### Requirement: Minijinja variable injection
The system SHALL preprocess snapshot definition files with minijinja variables provided by test configuration before parsing and execution.

#### Scenario: Variable substitution in request
- **WHEN** a definition file references a variable in the URL, headers, or body
- **THEN** the executed request uses the injected variable value

### Requirement: Snapshot output includes headers and body
The snapshot output MUST include full request and response transcripts with headers and body content.

#### Scenario: Snapshot capture
- **WHEN** a test executes successfully
- **THEN** the snapshot contains request headers, response headers, and response body

### Requirement: One file per snapshot output
The system MUST generate one snapshot output file per definition file, similar to codegen snapshot workflows.

#### Scenario: Snapshot generation per file
- **WHEN** the runner processes a definition file
- **THEN** exactly one snapshot output file is produced for that definition file

### Requirement: http_server snapshot coverage
The system SHALL include an HTTP snapshot definition and output for `xidlc-examples/api/http/http_server.idl`.

#### Scenario: http_server snapshot present
- **WHEN** snapshot tests are run
- **THEN** a snapshot exists for the http_server example

