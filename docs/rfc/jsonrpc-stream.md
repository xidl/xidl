# XIDL JSON-RPC Stream Mapping Specification (RFC Draft)

Reference specifications:

- <https://www.jsonrpc.org/specification>
- <https://www.omg.org/spec/DDS-RPC>

## 1. Scope

This document defines a streaming extension for XIDL JSON-RPC mapping.

Unlike base JSON-RPC 2.0 request/response, this profile supports long-lived
full-duplex stream message exchange.

This profile is XIDL-defined and transport-neutral.

## 2. Design Goals

- Keep JSON-RPC-compatible control messages (`method`, `id`, `error`).
- Allow server/client/bidirectional streaming in one uniform model.
- Prefer fewer restrictions than HTTP stream mapping.

## 3. Stream Frame Model

All stream frames are ordinary JSON-RPC messages whose `params` or `result`
carry stream envelope fields.

Common stream envelope:

```json
{
  "seq": 1,
  "kind": "next",
  "data": { "value": 1 }
}
```

Fields:

- `seq`: per-direction monotonic sequence (uint64)
- `kind`: `next|complete|cancel|error|ack|heartbeat`
- `data`: payload for `next`
- `err`: stream error payload when `kind = error`
- `meta`: optional metadata

## 4. Stream Method Declaration

Methods use stream annotations:

- `@server_stream`
- `@client_stream`
- `@bidi_stream`

They are mutually exclusive.

`@stream_codec("json")` is default for JSON-RPC stream profile.

## 5. Method Name and Message Flow

Base RPC method name remains:

- `{module_path}.{interface}.{method}`

For a stream method `chat`, control method names are derived as:

- `...chat.push`
- `...chat.close`
- `...chat.cancel`

Recommended flow:

1. Both sides exchange `push` notifications or requests (`kind=next`).
2. Either side sends `close` (`kind=complete`) to half-close.
3. Either side sends `cancel` (`kind=cancel`) to abort.

For `@bidi_stream`, both directions may send `push` concurrently.

## 6. Direction Mapping

Direction semantics reuse XIDL rules:

- request-side fields: `in` + `inout`
- response-side fields: `out` + `inout` + `return`

`@server_stream`:

- client sends stream messages
- server sends multiple `next`

`@client_stream`:

- client sends multiple `next`
- server sends final response / completion

`@bidi_stream`:

- both sides may send multiple `next`

## 7. Attribute Stream Mapping

Only attributes marked with `@server_stream` are stream-published.

Generated stream method:

- `watch_attribute_<name>`

Attribute stream payload recommendation:

- `value`
- `version`
- `ts`

Client-side attribute stream updates are not part of v1.

## 8. Error and Cancellation

Transport/protocol errors use JSON-RPC error object.

Stream business errors use envelope `kind=error` with `err` payload.

Recommended behavior:

- malformed frame -> JSON-RPC error (`-32600`/`-32602`)
- unknown stream method -> `-32601`
- internal failure -> `-32603` or server error range
- stream-level business failure -> `kind=error`

Cancellation:

- explicit `cancel`
- transport disconnect implies cancel for active streams on that channel

## 9. Ordering and Reliability

- `seq` must be monotonic per direction per stream.
- No global total ordering across streams is required.
- Delivery semantics are at-least-once unless runtime guarantees stronger mode.
- Consumers should tolerate duplicates with stream-level de-dup keys.

## 10. Validation Rules

Build-time:

- stream annotations are mutually exclusive
- duplicate stream control method names are invalid

Runtime:

- non-monotonic `seq` is protocol error
- sending `next` after terminal frame is invalid

## 11. Transport Notes

This RFC is transport-neutral. Common choices:

- TCP newline-delimited JSON
- WebSocket JSON text frames
- in-process channel transport

Full-duplex behavior does not depend on HTTP constraints in this profile.
Multiplexing (if needed) is transport/runtime responsibility, not protocol
handshake responsibility in this RFC.

## 12. Example (Bidirectional)

IDL:

```idl
interface Chat {
  @bidi_stream
  void room(in string room_id, in string text, out string from, out string text_out);
};
```

Push:

```json
{
  "jsonrpc": "2.0",
  "method": "Chat.room.push",
  "params": {
    "seq": 1,
    "kind": "next",
    "data": { "room_id": "r1", "text": "hello" }
  }
}
```

Complete:

```json
{
  "jsonrpc": "2.0",
  "method": "Chat.room.close",
  "params": { "seq": 9, "kind": "complete" }
}
```

## 13. Conformance Levels

Minimum v1:

- support `@server_stream`
- support `next|complete|error|cancel`
- enforce per-stream sequence rules

Full v1:

- support `@server_stream`, `@client_stream`, `@bidi_stream`
- support full-duplex concurrent push for `@bidi_stream`
- support attribute watch stream mapping
