# XIDL Repository Skill

Use this skill when you need to work effectively in the XIDL repository as an
agent or automation tool.

## Repository focus

XIDL is an OMG IDL-based toolchain that generates:

- Rust, C, C++, and TypeScript artifacts
- Rust Axum HTTP bindings
- Rust JSON-RPC bindings
- OpenAPI and OpenRPC schema output

## Start here

When you need orientation, read these docs first:

1. `docs/index.md`
2. `docs/user/xidlc.md`
3. `docs/user/idl.md`
4. `docs/user/http.md` or `docs/user/jsonrpc.md`
5. `docs/architecture.md`
6. `docs/plugin.md`

## Source-of-truth files

- CLI arguments: `xidlc/src/cli/`
- built-in target names and aliases: `xidlc/src/driver/lang.rs`
- generator implementations: `xidlc/src/generate/`
- Rust build integration: `xidl-build/src/lib.rs`
- HTTP runtime support: `xidl-rust-axum/`
- examples and schemas: `xidlc-examples/`
- formal transport rules: `docs/rfc/`

## Common commands

Generate Rust:

```bash
xidlc --lang rust --out-dir out api.idl
```

Generate Axum:

```bash
xidlc --lang rust-axum --out-dir out api.idl
```

Generate OpenAPI:

```bash
xidlc --lang openapi --out-dir out api.idl
```

Use Rust build integration:

```rust
xidl_build::Builder::new()
    .with_lang("rust")
    .compile(&["api.idl"])?;
```

## Important modeling rules

- `in`, `out`, and `inout` affect HTTP and JSON-RPC request/result shaping
- `@optional` preserves omission semantics and is important for HTTP/OpenAPI,
  Rust, and TypeScript generation
- HTTP behavior is defined by `docs/rfc/http.md`, `docs/rfc/http-stream.md`, and
  `docs/rfc/http-security.md`
- JSON-RPC behavior is defined by `docs/rfc/jsonrpc.md` and
  `docs/rfc/jsonrpc-stream.md`

## Plugin development

Plugins are external generators launched by `xidlc` as child processes.

Key expectations:

- executable naming convention: `xidl-<lang>`
- invocation includes `--endpoint <uri>`
- protocol is JSON-RPC 2.0
- required methods include parser properties and generate

Read `docs/plugin.md` before changing plugin-related behavior.

## Working style

- prefer implementation-backed claims over aspirational documentation
- cross-check docs with examples under `xidlc-examples/`
- when transport semantics are ambiguous, defer to the RFCs
- when target capability is unclear, inspect the generator module directly
