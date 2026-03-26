# Targets Reference

This page summarizes the built-in generators currently recognized by the XIDL
driver.

## Built-in targets

| Target         | Aliases                                    | Output style                        | Typical pairing               |
| -------------- | ------------------------------------------ | ----------------------------------- | ----------------------------- |
| `c`            | `cc`                                       | C source and headers                | native C projects             |
| `cpp`          | `c++`, `cxx`                               | C++ headers and sources             | native C++ projects           |
| `rust`         | `rs`                                       | Rust types and modules              | Rust libraries/applications   |
| `rust-jsonrpc` | `rust_jsonrpc`, `rs_jsonrpc`, `rs-jsonrpc` | Rust JSON-RPC bindings              | `xidl-jsonrpc`                |
| `rust-axum`    | `rust_axum`, `axum`, `rs_axum`, `rs-axum`  | Rust HTTP/stream bindings           | `xidl-rust-axum`              |
| `typescript`   | `ts`                                       | TypeScript declarations and schemas | TypeScript clients/tools      |
| `openapi`      | none                                       | `openapi.json`                      | API documentation/publishing  |
| `openrpc`      | `open-rpc`                                 | `openrpc.json`                      | JSON-RPC documentation        |
| `hir`          | none                                       | HIR artifact                        | debugging or advanced tooling |
| `typed-ast`    | `typed_ast`                                | typed AST artifact                  | debugging or advanced tooling |

## Client/server flags

Some generators respond to `--client` and `--server`.

Common examples:

- `rust-axum`
- `rust-jsonrpc`

If a generator does not support the distinction, the flags may be ignored or
have no visible effect.

## Single-file schema targets

`xidl-build::Builder::with_output_filename` is currently intended for:

- `openapi`
- `openrpc`

## Where to look next

- practical workflow: [Using xidlc](../user/xidlc.md)
- Rust build integration: [Using xidl-build in Rust](../user/xidl-build.md)
- HTTP target behavior: [HTTP Guide](../user/http.md)
- JSON-RPC target behavior: [JSON-RPC Guide](../user/jsonrpc.md)
- Axum runtime note: [rust-axum](../rust-axum.md)
- Rust JSON-RPC note: [rust-jsonrpc](../rust-jsonrpc.md)
