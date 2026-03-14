## ADDED Requirements

### Requirement: Axum Server-Stream Projection
The system MUST generate Rust Axum server-side and client-side code for
`@server-stream` HTTP operations using the RFC-supported SSE profile.

#### Scenario: Server-stream method generates SSE service signature
- **WHEN** an IDL operation is annotated with `@server-stream`
- **THEN** Rust Axum generation MUST emit a service trait method that returns an
  SSE stream of the operation item type

#### Scenario: Server-stream client opens SSE reader
- **WHEN** Rust Axum client code is generated for an `@server-stream` operation
- **THEN** the generated client MUST send an HTTP request with `Accept:
  text/event-stream` and return a stream reader for decoded item values

### Requirement: Axum Client-Stream Projection
The system MUST generate Rust Axum server-side and client-side code for
`@client-stream` HTTP operations using NDJSON request streaming.

#### Scenario: Client-stream service accepts NDJSON input stream
- **WHEN** an IDL operation is annotated with `@client-stream`
- **THEN** Rust Axum generation MUST emit a service trait method whose request
  payload is an NDJSON item stream of the declared sequence item type

#### Scenario: Client-stream client sends NDJSON request body
- **WHEN** Rust Axum client code is generated for an `@client-stream` operation
- **THEN** the generated client MUST send `Content-Type:
  application/x-ndjson` and encode each stream item as one JSON line

### Requirement: Axum Stream Validation
The system MUST reject HTTP stream shapes that are outside the supported Axum
target matrix.

#### Scenario: Unsupported server-stream codec is rejected
- **WHEN** a Rust Axum target encounters an `@server-stream` operation whose
  effective codec is not SSE
- **THEN** code generation MUST fail with a validation error

#### Scenario: Unsupported client-stream codec is rejected
- **WHEN** a Rust Axum target encounters an `@client-stream` operation whose
  effective codec is not NDJSON
- **THEN** code generation MUST fail with a validation error

#### Scenario: Invalid stream http method is rejected
- **WHEN** a Rust Axum target encounters an `@server-stream` method that is not
  GET or an `@client-stream` method that is not POST
- **THEN** code generation MUST fail with a validation error
