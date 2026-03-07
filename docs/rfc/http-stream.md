# XIDL HTTP Stream Mapping Specification (RFC Draft)

Reference specifications:

- <https://www.rfc-editor.org/rfc/rfc9110>
- <https://html.spec.whatwg.org/multipage/server-sent-events.html>
- <https://www.rfc-editor.org/rfc/rfc7464>
- <https://www.omg.org/spec/DDS-RPC>

## 1. Scope

This document defines a streaming profile on top of the XIDL HTTP mapping.
It extends unary HTTP APIs with long-lived request/response streams.

This document defines:

- stream interaction model (server/client/bidi)
- method annotation model for stream operations
- HTTP wire profile and frame envelope
- lifecycle, cancellation, and error semantics

This document does not define:

- message broker transport
- reliability beyond HTTP transport guarantees
- exactly-once delivery

## 2. Terminology

- `stream operation`: an IDL operation annotated as streaming.
- `frame`: one logical message in a stream body.
- `upstream`: client -> server stream direction.
- `downstream`: server -> client stream direction.
- `half-close`: one direction is closed while the other is still open.

## 3. Stream Operation Model

### 3.1 Annotations

A method is treated as stream operation when it has:

- `@server_stream`: server streaming
- `@client_stream`: client streaming
- `@bidi_stream`: bidirectional streaming

Optional annotations:

- `@path("...")`: explicit route path (same normalization rules as HTTP RFC)
- `@stream_codec("ndjson" | "sse")`: payload framing codec

Defaults:

- HTTP method: `POST`
- codec: `ndjson`
- route: `/{method_name}` (or from `@path`)

Validation:

- `@server_stream`, `@client_stream`, and `@bidi_stream` are mutually exclusive.
- A stream method must declare exactly one of the three annotations above.

`@path` template compatibility:

- Stream methods reuse HTTP RFC route-template rules.
- Supported path template forms include:
  - `{name}` (path variable)
  - `{*name}` (catch-all path variable)
  - `{?name1,name2,...}` (query-template variables)
- Bound names must be resolved by request-side parameters using `@path`/`@query`.

Example:

```idl
@server_stream
@path("/files/{*path}{?lang,follow}")
void tail_file(
  @path("path") string path,
  @query("lang") string lang,
  @query("follow") boolean follow,
  out string line
);
```

### 3.2 Interaction Kinds

- `server`:
  - one request object
  - many response events
- `client`:
  - many request events
  - one final response object
- `bidi`:
  - many request events
  - many response events

Direction semantics still follow IDL `in/out/inout`:

- request-side fields: `in` + `inout`
- response-side fields: `out` + `inout` + `return` (if non-void)

## 4. Wire Profiles

## 4.1 Common HTTP Requirements

- `Content-Type` must match selected codec.
- `Transfer-Encoding: chunked` is used on HTTP/1.1 for indefinite bodies.
- HTTP/2 and HTTP/3 should be preferred for bidi streams.
- Stream responses should disable buffering at proxy/gateway layer.

Request headers:

- `x-xidl-stream-mode: server|client|bidi`
- `x-xidl-stream-version: 1`

## 4.2 NDJSON Codec (default)

MIME:

- request: `application/x-ndjson`
- response: `application/x-ndjson`

Each line is a JSON frame object.

Frame envelope:

```json
{ "t": "next", "seq": 1, "data": { "value": 42 } }
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

- valid only for `@server_stream`
- request body is normal unary JSON request (`application/json`)

## 5. Payload Mapping

For `next` frames, payload shape is derived from IDL outputs.

Request `next.data` shape:

- contains `in` and `inout` parameters only

Response `next.data` shape:

- contains `out`, `inout`, and `return` when return type is non-void

When only one logical field exists, object shape is still required.

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

## 8. Flow Control and Backpressure

Transport-level flow control uses HTTP/2 or HTTP/3 windowing.

Application-level recommendations:

- sender should cap in-flight `next` frames
- receiver may throttle by delaying reads
- optional `meta.credits` may be used for explicit pull-based flow control

`meta.credits` is advisory in v1 and not required for conformance.

## 9. Validation Rules

Build-time validation:

- `@stream_codec("sse")` is only valid with `@server_stream`
- duplicate stream routes after normalization are invalid
- non-POST stream methods are discouraged and should emit warnings

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

- Only attributes marked with `@server_stream` are mapped to change-notification streams.
- The generated watch operation is server-stream (`@server_stream`) only.
- Client-side attribute-change streaming is not supported in v1.
- Attribute change notification should use SSE in v1 (`@stream_codec("sse")`).

Generated operation shape:

- attribute `foo` with `@server_stream` maps to:
  - `watch_attribute_foo(...)`
- default route follows normal stream route rules unless overridden by `@path`.
- default codec for generated watch operation is SSE (`text/event-stream`).

Event payload recommendation:

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
  @server_stream
  attribute boolean online;
};
```

Generated watch stream (conceptually):

```idl
interface DeviceState {
  @server_stream
  @stream_codec("sse")
  void watch_attribute_online(out boolean value, out uint64 version, out string ts);
};
```

## 12. Examples

### 12.1 Server Stream (NDJSON)

IDL:

```idl
interface Metrics {
  @server_stream
  @path("/metrics/tail")
  void tail(@query("service") string service, out double cpu, out double mem);
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
  @client_stream
  UploadResult push(in string file_id, in sequence<octet> chunk);
};
```

Request frames:

```json
{ "t": "next", "seq": 1, "data": { "file_id": "f-1", "chunk": "...base64..." } }
{ "t": "next", "seq": 2, "data": { "file_id": "f-1", "chunk": "...base64..." } }
{ "t": "complete", "seq": 3 }
```

Final response:

```json
{ "return": { "ok": true, "bytes": 2097152 } }
```

### 12.3 Bidi Stream (NDJSON)

IDL:

```idl
interface Chat {
  @bidi_stream
  void room(in string room_id, in string text, out string from, out string text_out);
};
```

Both sides exchange `next` frames concurrently until one side sends `complete`
or `error`.

## 13. Conformance Levels

Minimum conformance (v1):

- support `@server_stream` + NDJSON codec
- support frame types `next|error|complete`
- enforce terminal and sequence rules

Full conformance (v1):

- support all stream modes (`server|client|bidi`)
- support cancellation semantics
- support SSE for server stream
