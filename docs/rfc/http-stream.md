# XIDL HTTP Stream Mapping Specification (RFC Draft)

Reference specifications:

- <https://www.rfc-editor.org/rfc/rfc9110>
- <https://html.spec.whatwg.org/multipage/server-sent-events.html>
- <https://www.rfc-editor.org/rfc/rfc7464>
- <https://www.omg.org/spec/DDS-RPC>

## 1. Scope

This document defines a streaming profile on top of the XIDL HTTP mapping. It
extends unary HTTP APIs with long-lived request or response streams.

This document defines:

- stream interaction model (server/client)
- method annotation model for stream operations
- HTTP wire profile and frame envelope
- stream security integration with the HTTP security mapping
- lifecycle, cancellation, and error semantics

This document does not define:

- message broker transport
- reliability beyond HTTP transport guarantees
- exactly-once delivery
- transport security negotiation

## 2. Terminology

- `stream operation`: an IDL operation annotated as streaming.
- `stream item`: one logical application value carried by a stream.
- `frame`: one logical message in a stream body.
- `upstream`: client -> server stream direction.
- `downstream`: server -> client stream direction.
- `effective security requirement`: the final authentication requirement that
  applies to a stream operation after interface-level inheritance and
  operation-level overrides are resolved.

## 3. Stream Operation Model

### 3.1 Annotations

A method is treated as stream operation when it has:

- `@server-stream`: server streaming
- `@client-stream`: client streaming

Optional annotations:

- `@path("...")`: explicit route path (same normalization rules as HTTP RFC)
- `@stream-codec("ndjson" | "sse")`: payload framing codec
- security annotations from the HTTP security mapping RFC, including
  `@no-security`, `@http-basic`, `@http-bearer`, `@api-key(...)`, and
  `@oauth2(...)`

Defaults:

- HTTP method: `POST`
- codec: `ndjson`
- route: `/{method_name}` (or from `@path`)

Validation:

- `@server-stream` and `@client-stream` are mutually exclusive.
- A stream method must declare exactly one of the two annotations above.
- `@client-stream` methods must declare exactly one streaming input parameter,
  and that parameter type must be `sequence<T>` for some item type `T`.
- `@server-stream` methods must declare a return type of `sequence<T>` for some
  item type `T`.

`@path` template compatibility:

- Stream methods reuse HTTP RFC route-template rules.
- Supported path template forms include:
  - `{name}` (path variable)
  - `{*name}` (catch-all path variable)
  - `{?name1,name2,...}` (query-template variables)
- Bound names must be resolved by request-side parameters using
  `@path`/`@query`.

Example:

```idl
@server-stream
@path("/files/{*path}{?lang,follow}")
sequence<string> tail_file(
  @path("path") string path,
  @query("lang") string lang,
  @query("follow") boolean follow
);
```

### 3.2 Interaction Kinds

- `server`:
  - one request object
  - many response items from `sequence<T>` return values
- `client`:
  - many request items from the single `sequence<T>` input parameter
  - one final response object

Sequence semantics:

- The stream item type is derived from `sequence<T>`.
- For `@client-stream`, `T` is the item type of the single streaming input
  parameter.
- For `@server-stream`, `T` is the item type of the `sequence<T>` return value.
- This RFC intentionally uses type-driven stream item semantics rather than a
  dedicated item annotation.

This RFC does not define bidirectional HTTP stream operations. True duplex
streaming is expected to use a separate WebSocket binding.

### 3.3 Security Model

Stream operations support the same security annotations and effective-requirement
rules defined by the HTTP security mapping RFC.

Supported annotations:

- `@no-security`
- `@http-basic`
- `@http-bearer`
- `@api-key(...)`
- `@oauth2(...)`

Inheritance and override rules:

- interface-level security annotations define the default security requirement
  for all stream operations in that interface
- operation-level security annotations replace inherited interface defaults
- `@no-security` on a stream operation clears inherited interface-level security
  requirements
- if no operation-level security annotations are present, the effective
  requirement is inherited from the interface

Stream-specific guidance:

- credentials are evaluated before the stream is established
- credentials are carried by the normal HTTP security mapping and are not part
  of stream `next.data` payloads
- long-lived streams do not redefine authentication semantics; token refresh,
  re-authentication, or session renewal remain implementation concerns
- security annotations add request-acceptance preconditions only and do not
  change stream item typing, codec selection, or frame sequencing

## 4. Wire Profiles

## 4.1 Common HTTP Requirements

- `Content-Type` must match selected codec.
- `Transfer-Encoding: chunked` is used on HTTP/1.1 for indefinite bodies.
- Stream responses should disable buffering at proxy/gateway layer.

Request headers:

- `x-xidl-stream-mode: server|client`
- `x-xidl-stream-version: 1`

Security headers and credentials follow the HTTP security mapping RFC. For
example, HTTP Basic and Bearer use `Authorization`, and API key credentials use
their declared header, cookie, or query location.

## 4.2 NDJSON Codec (default)

MIME:

- request: `application/x-ndjson`
- response: `application/x-ndjson`

Each line is a JSON frame object.

Frame envelope:

```json
{ "t": "next", "seq": 1, "data": 42 }
```

Fields:

- `t`: frame type, one of `next|error|complete|cancel|heartbeat`
- `seq`: monotonic uint64 per direction, starts at 1
- `data`: payload for `next`
- `error`: error object for `error`
- `meta`: optional metadata map

Rules:

- `next` may appear zero or more times.
- `complete` appears at most once and terminates that direction.
- `error` terminates the full stream immediately.
- `cancel` requests remote termination (best effort).

## 4.3 SSE Codec (server-stream only)

MIME:

- response: `text/event-stream`

SSE mapping:

- `event: next` + `data: <json payload>`
- `event: error` + `data: <json error>`
- `event: complete`

Constraints:

- valid only for `@server-stream`
- request body is normal unary JSON request (`application/json`)

## 5. Payload Mapping

For `next` frames, payload shape is derived from the stream item type `T`.

Request `next.data` shape for `@client-stream`:

- is one serialized item of type `T`, where the operation declares exactly one
  streaming input parameter of type `sequence<T>`

Response `next.data` shape for `@server-stream`:

- is one serialized item of type `T`, where the operation return type is
  `sequence<T>`

Generators targeting OpenAPI 3.2 should map the stream item type `T` to the
media type `itemSchema`.

Byte-stream mapping:

- a byte stream is modeled by choosing `T = octet`
- `@client-stream` byte streams therefore use an input parameter of type
  `sequence<octet>`
- `@server-stream` byte streams therefore use a return type of
  `sequence<octet>`
- octet chunk values are transported as JSON arrays of octets in NDJSON-based
  profiles unless a companion codec profile specifies another representation

Example:

```json
{ "return": 3 }
```

## 6. Lifecycle

## 6.1 Start

- Stream is established when server accepts HTTP request and returns `200`.
- `4xx/5xx` before first frame means stream was not established.

## 6.2 End States

A direction ends by:

- receiving `complete`
- receiving `error`
- transport close (EOF / reset)
- local cancellation (`cancel` or HTTP connection close)

Terminal rules:

- after `complete` or `error`, no additional frames are valid
- invalid extra frames should be ignored and logged

## 6.3 Cancellation

Client cancellation options:

- close request body (HTTP half-close if supported)
- close TCP/HTTP stream
- send `cancel` frame (NDJSON only)

Server cancellation options:

- send `error` frame
- close stream transport

## 7. Error Model

Error object:

```json
{
  "code": "INVALID_ARGUMENT",
  "message": "field x is required",
  "retryable": false,
  "details": { "field": "x" }
}
```

Mapping guidance:

- protocol decode error -> HTTP `400` before stream or `error` frame after start
- auth/authz failure -> HTTP `401/403`
- method/route not found -> HTTP `404`
- business failure during stream -> `error` frame
- overload/backpressure -> `error.code = "RESOURCE_EXHAUSTED"`

Security-specific guidance:

- missing or invalid credentials -> HTTP `401 Unauthorized`
- authenticated but insufficient privileges -> HTTP `403 Forbidden`
- when `401` is returned for HTTP-auth-based schemes, implementations SHOULD
  emit `WWW-Authenticate` where applicable
- if authentication fails before the first stream frame, the stream is not
  established and no stream frames are emitted

## 8. Flow Control and Backpressure

Transport-level flow control uses HTTP/2 or HTTP/3 windowing.

Application-level recommendations:

- sender should cap in-flight `next` frames
- receiver may throttle by delaying reads
- optional `meta.credits` may be used for explicit pull-based flow control

`meta.credits` is advisory in v1 and not required for conformance.

## 9. Validation Rules

Build-time validation:

- `@stream-codec("sse")` is only valid with `@server-stream`
- duplicate stream routes after normalization are invalid
- non-POST stream methods are discouraged and should emit warnings
- `@client-stream` methods must declare exactly one streaming input parameter
- that streaming input parameter must have type `sequence<T>`
- `@server-stream` methods must return `sequence<T>`
- `@no-security` must not be combined with other operation-level security
  annotations on the same stream operation
- security annotations on stream operations follow the same duplicate and
  argument validation rules defined by the HTTP security mapping RFC
- `@bidi-stream` is outside the scope of this RFC and should be rejected by
  conforming HTTP stream profiles

Runtime validation:

- unknown frame type -> stream protocol error
- non-monotonic `seq` -> stream protocol error
- malformed JSON line -> stream protocol error

## 10. Compatibility

- Unary HTTP operations remain unchanged.
- Stream and unary routes may coexist in one interface.
- Generated clients should expose stream APIs as async iterators/channels.
- Gateways that cannot pass streaming bodies should reject with `501`.

## 11. Attribute Stream Mapping

Attributes are not streamed by default.

Rule:

- Only attributes marked with `@server-stream` are mapped to change-notification
  streams.
- The generated watch operation is server-stream (`@server-stream`) only.
- Client-side attribute-change streaming is not supported in v1.
- Attribute change notification should use SSE in v1 (`@stream-codec("sse")`).
- The generated watch operation returns `sequence<T>`, where `T` is the event
  item type for that attribute stream.

Generated operation shape:

- attribute `foo` with `@server-stream` maps to:
  - `watch_attribute_foo(...): sequence<AttributeFooEvent>`
- default route follows normal stream route rules unless overridden by `@path`.
- default codec for generated watch operation is SSE (`text/event-stream`).

Event payload recommendation:

- define an event type such as `AttributeFooEvent`
- recommended fields on that event type:
  - `value`: current attribute value
  - `version`: monotonic version per attribute
  - `ts`: server timestamp

Behavior:

- the first pushed event should be a snapshot of current value
- later events are pushed only when value changes
- heartbeat is optional

Example:

```idl
interface DeviceState {
  @server-stream
  attribute boolean online;
};
```

Generated watch stream (conceptually):

```idl
struct OnlineEvent {
  boolean value;
  uint64 version;
  string ts;
};

interface DeviceState {
  @server-stream
  @stream-codec("sse")
  sequence<OnlineEvent> watch_attribute_online();
};
```

## 12. Examples

### 12.1 Server Stream (NDJSON)

IDL:

```idl
interface Metrics {
  @server-stream
  @path("/metrics/tail")
  sequence<MetricSample> tail(@query("service") string service);
};
```

Where `MetricSample` is a structured item type, for example:

```idl
struct MetricSample {
  double cpu;
  double mem;
};
```

Response frames:

```json
{ "t": "next", "seq": 1, "data": { "cpu": 0.61, "mem": 0.72 } }
{ "t": "next", "seq": 2, "data": { "cpu": 0.64, "mem": 0.71 } }
{ "t": "complete", "seq": 3 }
```

### 12.2 Client Stream (NDJSON)

IDL:

```idl
interface Upload {
  @client-stream
  UploadResult push(in sequence<octet> chunk);
};
```

Request frames:

```json
{ "t": "next", "seq": 1, "data": [1, 2, 3, 4] }
{ "t": "next", "seq": 2, "data": [5, 6, 7, 8] }
{ "t": "complete", "seq": 3 }
```

Final response:

```json
{ "return": { "ok": true, "bytes": 2097152 } }
```

### 12.3 Server Stream (Octets)

IDL:

```idl
interface Download {
  @server-stream
  sequence<octet> pull(@query("file") string file);
};
```

Response frames:

```json
{ "t": "next", "seq": 1, "data": [1, 2, 3, 4] }
{ "t": "next", "seq": 2, "data": [5, 6, 7, 8] }
{ "t": "complete", "seq": 3 }
```

## 13. Conformance Levels

Minimum conformance (v1):

- support `@server-stream` + NDJSON codec
- support frame types `next|error|complete`
- enforce terminal and sequence rules

Full conformance (v1):

- support both stream modes (`server|client`)
- support cancellation semantics
- support SSE for server stream
