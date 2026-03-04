# xidlc `rust-jsonrpc`

The `rust-jsonrpc` target generates Rust JSON-RPC service/client code from IDL.

## Minimal Flow

```bash
xidlc \
  --lang rust-jsonrpc \
  --out-dir out \
  your.idl
```

Then implement the generated trait and expose it with `xidl_jsonrpc::Server`.

## Example in this repo

Run server:

```bash
cargo run -p xidlc-examples --example jsonrpc_hello_world_server
```

Run client (in another terminal):

```bash
cargo run -p xidlc-examples --example jsonrpc_hello_world_client -- --name World
```
