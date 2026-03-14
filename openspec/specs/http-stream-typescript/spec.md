## ADDED Requirements

### Requirement: TypeScript Server-Stream Projection
The system MUST generate TypeScript client methods for `@server-stream`
operations using the Fetch-based SSE reader model.

#### Scenario: Server-stream method returns async iterable
- **WHEN** TypeScript code is generated for an `@server-stream` operation
- **THEN** the generated client method MUST return an async iterable of the
  stream item type

#### Scenario: Server-stream request advertises sse accept header
- **WHEN** a generated TypeScript client invokes an `@server-stream` operation
- **THEN** it MUST send `Accept: text/event-stream` unless the caller already
  provided an explicit value

### Requirement: TypeScript Client-Stream Projection
The system MUST generate TypeScript client methods for `@client-stream`
operations using NDJSON request streams over Fetch.

#### Scenario: Client-stream method accepts async iterable input
- **WHEN** TypeScript code is generated for an `@client-stream` operation
- **THEN** the generated client method MUST accept an async iterable of the
  declared stream item type

#### Scenario: Client-stream method encodes ndjson body
- **WHEN** a generated TypeScript client invokes an `@client-stream` operation
- **THEN** it MUST send `Content-Type: application/x-ndjson` and encode each
  yielded item as one JSON line in the request body stream

### Requirement: TypeScript Stream Validation
The system MUST reject HTTP stream definitions that are outside the supported
TypeScript client model.

#### Scenario: Bidirectional stream is rejected
- **WHEN** the TypeScript generator encounters an HTTP stream operation that
  requires bidirectional streaming
- **THEN** code generation MUST fail with a validation error

#### Scenario: Unsupported stream codec is rejected
- **WHEN** the TypeScript generator encounters a server-stream codec other than
  SSE or a client-stream codec other than NDJSON
- **THEN** code generation MUST fail with a validation error

#### Scenario: Client-stream with non-body inputs is rejected
- **WHEN** the TypeScript generator encounters an `@client-stream` operation
  that requires path, query, header, or cookie request inputs
- **THEN** code generation MUST fail with a validation error
