# Architecture

This document explains how XIDL turns IDL input into generated artifacts and
which crates own each stage of the pipeline.

## High-level flow

```d2
source -> ast: tree-sitter parse
ast -> typed_ast: typed AST conversion
typed_ast -> hir: semantic lowering
hir -> driver: generator session
driver -> builtins: built-in generators
driver -> plugins: external JSON-RPC generators
builtins -> outputs: files / schemas / runtime bindings
plugins -> outputs
```

## Pipeline stages

### 1. CLI and driver

Main entry points:

- `xidlc/src/main.rs`
- `xidlc/src/cli/`
- `xidlc/src/driver/`

Responsibilities:

- parse command-line arguments
- select the generator target
- decide output mode and directory
- run built-in generators or external plugins

The current generation arguments are centered on `ArgsGenerate`, which carries
`lang`, `out_dir`, `client`, `server`, `dry_run`, and input files.

### 2. Parsing and diagnostics

Key crates and modules:

- `xidl-parser/`
- `xidlc/src/diagnostic/`

Responsibilities:

- parse IDL source
- surface syntax and semantic errors with highlighted diagnostics
- build the intermediate structures needed for later lowering

### 3. Lowering to HIR

The repository uses a higher-level semantic representation for generator logic.
Generators generally operate on HIR rather than raw parser output because HIR
captures declarations, annotations, and transport-relevant semantics in a more
stable form.

### 4. Generation

Built-in generators live under `xidlc/src/generate/`.

Important modules include:

- `c/`
- `cpp/`
- `rust/`
- `rust_axum/`
- `rust_jsonrpc/`
- `typescript/`
- `openapi/`
- `openrpc/`

Responsibilities:

- map HIR into target-specific types, traits, routes, schemas, or files
- apply target-specific annotation handling
- emit one or more artifacts back to the driver

### 5. Runtime pairings

Some generators emit code that is meant to work with runtime crates:

- `rust-axum` pairs with `xidl-rust-axum`
- `rust-jsonrpc` pairs with `xidl-jsonrpc`

These runtime crates hold request/response types, server/client helpers,
transport helpers, and error models that generated code depends on.

## Crate responsibilities

### `xidlc`

Owns:

- CLI entry point
- generation driver
- built-in generators
- formatting support
- diagnostic presentation

### `xidl-build`

Owns:

- Rust build-script integration
- `Builder` API that wraps the same internal generation driver

Use this crate when you want Cargo-driven generation from `build.rs`.

### `xidl-rust-axum`

Owns:

- HTTP and stream runtime support for generated Rust Axum bindings
- request wrappers, auth helpers, stream helpers, and error handling

### `xidl-jsonrpc`

Owns:

- JSON-RPC runtime abstractions and server/client infrastructure used by
  generated Rust JSON-RPC output and plugin transport

### `xidlc-examples`

Owns:

- runnable examples
- generated schema snapshots
- transport-focused tests that are useful as documentation anchors

## Generator selection and aliases

Built-in target resolution is defined in `xidlc/src/driver/lang.rs`.

Examples:

- `rust`, `rs`
- `rust-jsonrpc`, `rust_jsonrpc`
- `rust-axum`, `rust_axum`, `axum`
- `typescript`, `ts`
- `openapi`
- `openrpc`

Anything else is treated as a custom plugin target.

## Plugin model

External generators use the same driver pipeline up to HIR, then hand off to a
JSON-RPC plugin process. The driver is responsible for:

- launching the plugin
- providing an endpoint with `--endpoint`
- sending parser properties and generation requests
- writing returned files

Read [Plugin Development](plugin.md) for the protocol contract.

## Documentation and examples as architecture aids

When you are tracing a feature through the system, these files are usually the
fastest route:

- transport semantics: `docs/rfc/`
- target docs: `xidlc/doc/`
- live examples: `xidlc-examples/api/` and `xidlc-examples/examples/`
- implementation: `xidlc/src/generate/`

## Typical contributor workflow

1. Reproduce behavior with an example IDL file.
2. Locate the target generator module under `xidlc/src/generate/`.
3. Check whether the runtime crate also needs changes.
4. Update examples, tests, and docs together when behavior changes.
