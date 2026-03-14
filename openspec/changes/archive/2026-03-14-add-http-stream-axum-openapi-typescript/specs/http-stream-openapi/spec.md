## ADDED Requirements

### Requirement: OpenAPI Version Upgrade
The system MUST emit OpenAPI 3.2.0 documents for generated HTTP API
descriptions.

#### Scenario: Generated document declares OpenAPI 3.2.0
- **WHEN** the OpenAPI generator renders an API document
- **THEN** the top-level `openapi` field MUST be `3.2.0`

### Requirement: OpenAPI Server-Stream Projection
The system MUST describe HTTP `@server_stream` operations in generated OpenAPI
using the SSE media type and the stream item schema.

#### Scenario: Server-stream response uses text event stream item schema
- **WHEN** the OpenAPI generator renders an `@server_stream` operation
- **THEN** the success response MUST use `text/event-stream` and MUST describe
  the stream item type with OpenAPI 3.2.0 `itemSchema`

#### Scenario: Server-stream operation preserves request bindings
- **WHEN** the OpenAPI generator renders an `@server_stream` operation with
  path or query bindings
- **THEN** the operation MUST preserve those bindings as OpenAPI parameters in
  the generated path item

### Requirement: OpenAPI Client-Stream Projection
The system MUST describe HTTP `@client_stream` operations in generated OpenAPI
using the NDJSON media type and the stream item schema.

#### Scenario: Client-stream request body uses ndjson item schema
- **WHEN** the OpenAPI generator renders an `@client_stream` operation
- **THEN** the request body MUST use `application/x-ndjson` and MUST describe
  the streamed request item type with OpenAPI 3.2.0 `itemSchema`

#### Scenario: Client-stream final response stays unary
- **WHEN** the OpenAPI generator renders an `@client_stream` operation
- **THEN** the success response MUST describe the final unary response payload
  instead of an array or stream body

### Requirement: OpenAPI Stream Validation
The system MUST reject HTTP stream definitions that cannot be represented by
the supported OpenAPI projection.

#### Scenario: Unsupported stream codec is rejected
- **WHEN** the OpenAPI generator encounters a stream operation whose codec is
  outside the supported SSE or NDJSON projection
- **THEN** generation MUST fail with a validation error

#### Scenario: Unsupported stream method is rejected
- **WHEN** the OpenAPI generator encounters an `@server_stream` operation that
  is not GET or an `@client_stream` operation that is not POST
- **THEN** generation MUST fail with a validation error
