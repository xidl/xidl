# xidlc `rust-axum`

The `rust-axum` target generates Rust HTTP bindings for Axum-based services and
clients.

## Minimal flow

```bash
xidlc gen --out-dir generated rust-axum hello_world.idl
```

Generate OpenAPI alongside it when you need a schema document:

```bash
xidlc gen --out-dir generated openapi hello_world.idl
```

## What gets generated

For each interface, the target currently emits:

- a Rust trait for business logic
- a server wrapper that wires routes and request parsing
- a client wrapper
- transport helper types used by generated adapters

## Minimal server shape

```rust
mod imp;

use imp::HelloWorld;
use imp::HelloWorldServer;

struct HelloWorldImpl;

#[async_trait::async_trait]
impl HelloWorld for HelloWorldImpl {
    async fn say_hello(
        &self,
        name: String,
    ) -> Result<(), xidl_rust_axum::Error> {
        println!("hello {}", name);
        Ok(())
    }
}
```

## Runtime pairing

Generated code is designed to work with `xidl-rust-axum`.

Use this target with:

- [HTTP Guide](user/http.md)
- [HTTP RFC](rfc/http.md)
- [HTTP Stream RFC](rfc/http-stream.md)
- [HTTP Security RFC](rfc/http-security.md)
