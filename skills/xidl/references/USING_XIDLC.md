# Using xidlc: CLI & Integration

`xidlc` is the core compiler and generator for XIDL.

## 1. CLI Usage

### Formatting

Always format your IDL files to maintain consistency.

```bash
xidlc fmt --inplace api.idl
```

### Code Generation

The `gen` command transforms IDL into various targets.

```bash
xidlc gen --out-dir <output> <target> <input.idl>
```

**Common Targets:**

- `rust`: Pure Rust data types.
- `rust-axum`: Rust server stubs using the Axum framework.
- `rust-jsonrpc`: Rust JSON-RPC client/server.
- `typescript`: TypeScript interfaces and classes.
- `openapi`: OpenAPI 3.x JSON specification.
- `go`: Go types and client/server.

## 2. Rust Build Integration (xidl-build)

Use the `xidl-build` crate in your `build.rs` for automatic generation.

### Cargo.toml

```toml
[build-dependencies]
xidl_build = "0.31.0"
```

### build.rs

```rust
use xidl_build::Builder;

fn main() {
    println!("cargo:rerun-if-changed=api.idl");
    Builder::new()
        .with_lang("rust-axum")
        .compile(&["api.idl"])
        .unwrap();
}
```

### Source Usage

Include the generated file in your Rust code:

```rust
mod api {
    include!(concat!(env!("OUT_DIR"), "/api.rs"));
}
```
