# IDL4 to Rust JSON-RPC mapping

This document describes xidlc’s Rust JSON-RPC generator. It extends the Rust
mapping rules in `idl2rust.md`, but only targets interfaces and modules. Other
IDL definitions are ignored in JSON-RPC output.

## General

- Based on Rust mapping with JSON-RPC-specific overrides.
- Only `module` and `interface` definitions are emitted.
- IDL modules map to Rust modules of the same name.
- Method names use the raw IDL identifier for the JSON-RPC method name, and the
  Rust-escaped identifier for the Rust trait method.
- Fully-qualified JSON-RPC method name is:
  `module_path.join(".") + "." + interface + "." + method`.

## Type Mapping Overrides

- `any`, `object`, and `valuebase` map to `serde_json::Value`.
- `sequence<T>` maps to `Vec<T>`.
- `map<K, V>` maps to `BTreeMap<K, V>`.
- `string` / `wstring` map to `String`.
- `fixed` maps to `f64`.
- Other scalar and enum mappings follow `idl2rust.md`.

All interface parameters are passed by value in JSON-RPC (no `&` or `&mut`), and
`in/out/inout` attributes are currently ignored.

## Interfaces

Each IDL interface produces:

- A Rust trait with JSON-RPC error handling:
  `fn method(...) -> Result<Ret, xidl_jsonrpc::Error>`.
- A server wrapper `InterfaceServer<T>` implementing `xidl_jsonrpc::Handler`.
- A client wrapper `InterfaceClient<R, W>` that implements the trait and makes
  JSON-RPC calls.
- For each method, a `Params` struct with `#[derive(Serialize, Deserialize)]`
  that bundles method arguments.

Attributes map to RPC methods:

- `readonly attribute foo` -> RPC method `foo`.
- `attribute foo` -> RPC methods `foo` and `set_foo`.
- `readonly` attributes with `raises` are currently skipped.

### Example

```idl
module math {
    interface Calc {
        long add(in long a, in long b);
        readonly attribute long version;
        attribute string name;
    };
}
```

```rust
pub trait Calc {
    fn add(&self, a: i32, b: i32) -> Result<i32, xidl_jsonrpc::Error>;
    fn version(&self) -> Result<i32, xidl_jsonrpc::Error>;
    fn name(&self) -> Result<String, xidl_jsonrpc::Error>;
    fn set_name(&self, value: String) -> Result<(), xidl_jsonrpc::Error>;
}

#[derive(Serialize, Deserialize)]
struct CalcAddParams {
    a: i32,
    b: i32,
}

pub struct CalcServer<T> {
    inner: T,
}

impl<T> xidl_jsonrpc::Handler for CalcServer<T>
where
    T: Calc,
{
    fn handle(&self, method: &str, params: Value) -> Result<Value, xidl_jsonrpc::Error> {
        match method {
            "math.Calc.add" => { /* ... */ }
            "math.Calc.version" => { /* ... */ }
            "math.Calc.name" => { /* ... */ }
            "math.Calc.set_name" => { /* ... */ }
            _ => Err(xidl_jsonrpc::Error::method_not_found(method)),
        }
    }
}
```

## Stream Methods

Stream behavior is selected with method annotations (mutually exclusive):

| Annotation       | Client API                                                               | Server API                                                               | Wire                                       |
| ---------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------ | ------------------------------------------ |
| `@server_stream` | `fn m(req) -> Result<BoxStream<'static, Item>, Error>`                   | `async fn m<'a>(&'a self, req) -> Result<BoxStream<'a, Item>, Error>`    | server stream (one request, N typed items) |
| `@client_stream` | `fn m(BoxStream<'a, Value>) -> Result<Ret, Error>`                       | same                                                                     | client stream                              |
| `@bidi_stream`   | `fn m(BoxStream<'static, Req>) -> Result<BoxStream<'static, Resp>, Err>` | `async fn m(&self, BoxStream<'static, Req>) -> Result<BoxStream, Error>` | typed bidirectional stream                 |

`@server_stream` is a typed push stream: call once with the request, get back a
stream of the IDL return type. The return type is the stream item type.

```idl
struct Message { long seq; string body; };
struct Request { string topic; };

interface Rpc {
    @server_stream
    Message subscribe(in Request req);
};
```

```rust
// server
async fn subscribe(
    &self,
    req: Request,
) -> Result<xidl_jsonrpc::stream::BoxStream<'static, Message>, xidl_jsonrpc::Error> {
    let topic = req.topic;
    Ok(Box::pin(async_stream::try_stream! {
        for msg in self.bus.subscribe(topic) {
            yield msg;
        }
    }))
}

// client
let mut stream = rpc.subscribe(Request { topic: "t".into() }).await?;
while let Some(msg) = stream.try_next().await? {
    // msg: Message
}
```

`@bidi_stream` is a fully typed bidirectional stream: pass an input stream of
requests, get back an output stream of responses:

```idl
struct Message { string text; };
struct Reply { string text; };

interface Chat {
    @bidi_stream
    Reply echo(in Message req);
};
```

```rust
// server
async fn echo(
    &self,
    mut stream: xidl_jsonrpc::stream::BoxStream<'static, Message>,
) -> Result<xidl_jsonrpc::stream::BoxStream<'static, Reply>, xidl_jsonrpc::Error> {
    Ok(xidl_jsonrpc::stream::boxed(async_stream::try_stream! {
        while let Some(msg) = stream.next().await {
            let msg = msg?;
            yield Reply { text: format!("echo: {}", msg.text) };
        }
    }))
}

// client
let in_stream = xidl_jsonrpc::stream::boxed(async_stream::try_stream! {
    yield Message { text: "hello".into() };
});
let mut out_stream = client.echo(in_stream).await?;
while let Some(reply) = out_stream.next().await {
    let reply = reply?;
}
```

## Notes and Limitations

- JSON-RPC output only includes interfaces; structs, enums, unions, etc. are
  expected to be available from the Rust generator output or other crates.
- `raises` clauses are ignored and do not affect signatures.
- `@derive(...)` annotations are not used in JSON-RPC output (only internal
  params structs derive `Serialize`/`Deserialize`).
