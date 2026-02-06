# XIDL

XIDL (eXtensible IDL) is an OMG IDL-based code generator.
It supports targets such as `c`, `cpp`, `rust`, `rust-axum`, and `rust-jsonrpc`.
XIDL is plugin-capable and uses JSON-RPC as the plugin communication protocol.

## Features

- Multi-language code generation from IDL
- Layered generation pipeline (`typed_ast` / `hir` / target code)
- Pluggable codegen engines via JSON-RPC
- Built-in formatting for IDL / Rust / C++ / TypeScript

## Quick Start

```bash
# Build
cargo build

# Run tests
cargo test

# Generate code (example)
cargo run -p xidlc -- -l rust -o out your.idl
```

## Built-in Targets (current)

- `c`
- `cpp`
- `rust`
- `rust-jsonrpc`
- `rust-axum`
- `ts` / `typescript`

## Plugin Model

- During generation, `xidlc` starts a codegen engine based on the target language.
- Built-in engines run in-process; unknown targets can be handled by external `xidl-<lang>` plugins.
- Plugins communicate with the host via JSON-RPC; see `xidlc/src/jsonrpc/ipc.idl` for the core interface.

## Repository Layout

- `xidlc/`: compiler CLI and language generators
- `xidl-parser/`: IDL parsing, typed AST, and HIR
- `xidl-jsonrpc/`: JSON-RPC communication library
- `xidl-xcdr/`: XCDR-related implementation
- `tree-sitter-idl/`: IDL grammar and queries
