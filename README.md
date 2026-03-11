# XIDL

XIDL (eXtensible IDL) is an OMG IDL-based code generator. It supports targets
such as `c`, `cpp`, `rust`, `rust-axum`, and `rust-jsonrpc`. XIDL is
plugin-capable and uses JSON-RPC as the plugin communication protocol.

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
xidlc -l rust -o out your.idl
```

## Docs (Docusaurus)

```bash
cd website
pnpm install
pnpm start
```

## Built-in Targets (current)

- `c`
- `cpp`
- `rust`
- `rust-jsonrpc`
- `rust-axum`
- `ts` / `typescript`

## Plugin Model

- During generation, `xidlc` starts a codegen engine based on the target
  language.
- Built-in engines run in-process; unknown targets can be handled by external
  `xidl-<lang>` plugins.
- Plugins communicate with the host via JSON-RPC; see
  `xidlc/src/jsonrpc/ipc.idl` for the core interface.

## Repository Layout

- `xidl-parser-derive`: tree-sitter helper derive.
- `xidl-parser`: tree-sitter parser.
- `xidlc`: eXtensible IDL compiler.
- `xidl-build`: xidlc builder derive.
- `xidl-jsonrpc`: xidlc rust jsonrpc framework.
- `xidl-rust-axum`: xidlc rust axum codegen framework.
- `xidl-typeobject`: omg dds typeobject.
- `xidl-xcdr`: omg dds xcdr.
- `xidlc-examples`: examples
