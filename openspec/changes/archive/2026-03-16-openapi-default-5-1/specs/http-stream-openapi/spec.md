## MODIFIED Requirements

### Requirement: OpenAPI Version Upgrade

The system MUST emit OpenAPI 3.1 documents by default and MUST emit OpenAPI
3.2.0 when the generated document requires 3.2-only features (for example,
stream `itemSchema`).

#### Scenario: Generated document declares OpenAPI 3.1 by default

- **WHEN** the OpenAPI generator renders an API document that does not use any
  3.2-only features
- **THEN** the top-level `openapi` field MUST be `3.1.0`

#### Scenario: Generated document declares OpenAPI 3.2.0 when required

- **WHEN** the OpenAPI generator renders an API document that uses any 3.2-only
  features
- **THEN** the top-level `openapi` field MUST be `3.2.0`

### Requirement: OpenAPI Server-Stream Projection

The system MUST describe HTTP `@server-stream` operations in generated OpenAPI
using the SSE media type and the stream item schema.

#### Scenario: Server-stream response uses text event stream item schema

- **WHEN** the OpenAPI generator renders an `@server-stream` operation
- **THEN** the success response MUST use `text/event-stream` and MUST describe
  the stream item type with OpenAPI 3.2.0 `itemSchema`

#### Scenario: Server-stream operation preserves request bindings

- **WHEN** the OpenAPI generator renders an `@server-stream` operation with path
  or query bindings
- **THEN** the operation MUST preserve those bindings as OpenAPI parameters in
  the generated path item

### Requirement: OpenAPI Client-Stream Projection

The system MUST describe HTTP `@client-stream` operations in generated OpenAPI
using the NDJSON media type and the stream item schema.

#### Scenario: Client-stream request body uses ndjson item schema

- **WHEN** the OpenAPI generator renders an `@client-stream` operation
- **THEN** the request body MUST use `application/x-ndjson` and MUST describe
  the streamed request item type with OpenAPI 3.2.0 `itemSchema`

#### Scenario: Client-stream final response stays unary

- **WHEN** the OpenAPI generator renders an `@client-stream` operation
- **THEN** the success response MUST describe the final unary response payload
  instead of an array or stream body
