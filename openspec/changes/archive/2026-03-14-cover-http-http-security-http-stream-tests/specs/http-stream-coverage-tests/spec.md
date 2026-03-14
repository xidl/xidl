## ADDED Requirements

### Requirement: HTTP Stream Generator Coverage Matrix
The `xidlc` test suite MUST cover the published HTTP stream support matrix
across the relevant generator targets using explicit positive fixtures.

#### Scenario: Supported stream directions are covered
- **WHEN** a target supports server-stream, client-stream, or bidi-stream
  behavior
- **THEN** `xidlc/tests` MUST include focused fixtures that verify the
  generated output for each supported stream direction on that target

#### Scenario: Stream binding and response semantics are covered
- **WHEN** a streamed operation uses request bindings or a final unary response
- **THEN** the test suite MUST include fixtures that verify those semantics in
  the generated target output

### Requirement: HTTP Stream Invalid Coverage
The `xidlc` test suite MUST assert unsupported stream combinations and
directional constraints explicitly.

#### Scenario: Unsupported stream method or codec is asserted
- **WHEN** a stream operation uses an invalid method or unsupported codec for a
  target
- **THEN** the test suite MUST include a targeted assertion on the resulting
  validation error or panic path

#### Scenario: Unsupported stream shapes are asserted
- **WHEN** a target rejects bidi-stream support, non-body client-stream inputs,
  or mutually exclusive stream annotations
- **THEN** the test suite MUST include targeted assertions for those rejection
  paths

### Requirement: HTTP Stream Example Integration Coverage
The `xidlc-examples` package MUST include end-to-end tests for representative
HTTP and JSON-RPC stream flows generated from the example IDLs.

#### Scenario: Example tests exercise generated stream clients and servers
- **WHEN** `xidlc-examples` validates generated stream behavior
- **THEN** its integration tests MUST execute server-stream, client-stream, and
  bidi-stream flows that prove the generated code interoperates correctly

#### Scenario: Example tests cover both network and local transport paths where relevant
- **WHEN** an example stream API supports more than one transport style or
  runtime mode
- **THEN** `xidlc-examples/tests` MUST include the representative paths needed
  to protect that behavior from regression
