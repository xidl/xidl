# XIDL

XIDL is a code generation toolchain based on OMG IDL.

## Quick Start

```bash
# Build
cargo build

# Test
cargo test

# Generate code
xidlc -l rust -o out your.idl
```

## Docs

- [Plugin Development](plugin.md)
- [xidlc / axum](rust-axum.md)
- [xidlc / http](http.md)
- [xidlc / jsonrpc](rust-jsonrpc.md)
