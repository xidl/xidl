## ADDED Requirements

### Requirement: Stream Snapshot Matrix Coverage
The `xidlc` test suite MUST include stream-oriented snapshot fixtures that cover
each supported HTTP stream shape for every target that advertises that shape.

#### Scenario: Supported stream fixtures exist for each advertised target
- **WHEN** the repository defines stream code generation support for `axum`,
  `openapi`, or `ts`
- **THEN** `xidlc/tests/<target>/` MUST contain dedicated fixtures covering the
  supported server-stream and client-stream shapes for that target

#### Scenario: Stream fixtures isolate distinct behavior classes
- **WHEN** a new supported stream behavior is added to the test suite
- **THEN** it MUST be represented by a focused fixture whose filename identifies
  the stream concern being exercised instead of being folded into an unrelated
  catch-all fixture

### Requirement: Stream Unsupported-Matrix Validation
The `xidlc` test suite MUST assert validation failures for HTTP stream
combinations that are outside the supported target matrix.

#### Scenario: Unsupported stream codec is tested explicitly
- **WHEN** a target rejects a stream operation because the effective codec is
  unsupported
- **THEN** the test suite MUST include a dedicated invalid fixture and an
  assertion on the resulting validation error

#### Scenario: Unsupported stream method or shape is tested explicitly
- **WHEN** a target rejects a stream operation because of an invalid HTTP method
  or unsupported directionality such as bidi-stream
- **THEN** the test suite MUST include a dedicated invalid fixture and an
  assertion on the resulting validation error

### Requirement: Stream Coverage Includes Binding Interactions
The `xidlc` test suite MUST cover stream fixtures that exercise HTTP binding
behavior alongside streaming semantics.

#### Scenario: Server-stream fixtures preserve request bindings
- **WHEN** a server-stream fixture uses path, query, or header bindings
- **THEN** snapshot coverage MUST verify the generated target output still
  reflects those bindings

#### Scenario: Client-stream fixtures preserve final unary response handling
- **WHEN** a client-stream fixture streams request items and returns a final
  response payload
- **THEN** snapshot coverage MUST verify that the generated target output keeps
  the response unary rather than projecting it as a streamed payload
