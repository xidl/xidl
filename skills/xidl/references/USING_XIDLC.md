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

## 3. Library Manifests (`xidl.toml`)

A directory holding `xidl.toml` is a library: pass the directory to `xidlc`
instead of the individual files and every declared input is compiled.

```toml
name = "timestamp"
version = "1.2.3"
compiler = ">=0.100"

[dependencies]
"timestamp" = { src = "github.com/xidl/timestamp", tag = "v1.2.3" }
"local-timestamp" = { path = "../timestamp" }
```

- Inputs default to the `*.idl` files beside the manifest; `files` and `exclude`
  glob patterns refine that set.
- Dependencies are fetched with `git` into `~/.xidl/pkg/<host>/<name>-<version>`
  and mounted by name, so contracts consume them with
  `#include "timestamp/lib.idl"`.
- A `path` dependency mounts a local directory relative to the manifest and
  bypasses the package cache, which keeps editable sibling libraries in sync.
- `compiler` is a version range; when the running `xidlc` is outside it, the
  newest matching release is installed into `~/.xidl/toolchain/<version>/` and
  used instead.
- `xidl-build` accepts a library directory in `compile(&[...])` too; the
  `bundle` backend resolves it in-process.

Full reference: `website/src/content/docs/docs/xidl-toml.mdx`.
