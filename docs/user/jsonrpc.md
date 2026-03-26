# JSON-RPC Guide

This guide explains the JSON-RPC family of XIDL features. It summarizes normal
usage and points to the formal RFCs when exact semantics matter.

Normative references:

- [JSON-RPC RFC](../rfc/jsonrpc.md)
- [JSON-RPC Stream RFC](../rfc/jsonrpc-stream.md)

## What the JSON-RPC stack includes

In this repository, the JSON-RPC family covers:

1. unary JSON-RPC request/response mapping
2. JSON-RPC stream mapping
3. runtime- and deployment-level security expectations

Typical outputs are:

- Rust JSON-RPC server/client code with `rust-jsonrpc`
- OpenRPC schema output with `openrpc`

## Basic JSON-RPC mapping

JSON-RPC method names are derived from module, interface, and operation names.

```idl
module math {
    interface Calc {
        long add(in long a, in long b);
    };
};
```

The JSON-RPC method name becomes `math.Calc.add`.

Practical mapping rules:

- request `params` are encoded as an object
- `in` and `inout` values appear on the request side
- return values, `out`, and `inout` values appear in the result object
- generated clients and servers use `xidl-jsonrpc`

## Example

```idl
interface SmartCityRpcApi {
    string quote_trip(
        in string rider_id,
        in string zone_id,
        out long amount_cents,
        out string currency
    );
};
```

This shape naturally maps to a JSON-RPC params object and a result object.

## Generated artifacts

With `rust-jsonrpc`, interfaces currently generate:

- a Rust trait
- a server wrapper implementing the JSON-RPC handler contract
- a client wrapper
- per-method params structs

With `openrpc`, the compiler emits `openrpc.json`.

## JSON-RPC Stream

The JSON-RPC stream profile extends the unary model with stream-oriented
messages.

Current stream annotations:

- `@server_stream`
- `@client_stream`
- `@bidi_stream`

Practical notes from the current repo:

- the stream profile is transport-neutral at the RFC level
- examples in this repository demonstrate server, client, and bidirectional
  modes
- full-duplex behavior and framing details are runtime concerns layered on top
  of the JSON-RPC message model

Example:

```idl
interface CityJsonrpcStreamApi {
    @server_stream
    void alerts(
        in string district,
        out string message
    );
};
```

## JSON-RPC security expectations

There is no separate JSON-RPC security RFC in this repository today. In
practice, JSON-RPC security is determined by the surrounding transport and
deployment model.

That means:

- auth may be handled by the HTTP layer, websocket handshake, IPC boundary, or
  process boundary
- the JSON-RPC user docs should describe operational expectations, not invent a
  new normative scheme
- if you need formal auth semantics today, document them in your runtime or
  transport layer

## Suggested workflow

1. Write interfaces and data types in IDL.
2. Generate `rust-jsonrpc` output when you need runnable Rust bindings.
3. Generate `openrpc` output when you need a schema document.
4. Use the RFC when result shaping, stream framing, or direction rules matter.

## Related material

- [HTTP Guide](http.md)
- [Targets Reference](../reference/targets.md)
- [Rust JSON-RPC target note](../rust-jsonrpc.md)
