# Development Docs

This section is for contributors, plugin authors, and maintainers.

## Read this section when you need to

- understand the crate layout
- trace the path from IDL source to generated output
- add or modify a generator
- build an external plugin
- understand the documentation coverage and remaining maintenance concerns

## Recommended reading order

1. [Architecture](../architecture.md)
2. [Plugin Development](../plugin.md)
3. [Documentation Audit](doc-audit.md)
4. [Axum Request Extractor Plan](axum-request-extractor-plan.md)
5. [Axum Unary Parameter Expansion TODO](axum-request-extractor-todo.md)

## Repository map

- `xidlc/`: compiler, CLI, formatters, generators
- `xidl-build/`: Rust build-script integration
- `xidl-rust-axum/`: HTTP/stream runtime support
- `xidl-jsonrpc/`: JSON-RPC runtime support
- `xidlc-examples/`: runnable examples and schema snapshots
- `docs/`: published documentation and RFCs
