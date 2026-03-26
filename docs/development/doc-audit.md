# Documentation Audit

This page records the documentation audit used to design the current docs
restructure.

## Existing sources reviewed

### Published docs

- `README.md`
- `docs/rust-axum.md`
- `docs/rust-jsonrpc.md`
- `docs/openapi.md`
- `docs/pragma.md`
- `docs/xidl-extend.md`
- `docs/architecture.md`
- `docs/plugin.md`
- `docs/rfc/*.md`

### Implementation-backed sources

- `xidlc/src/cli/`
- `xidlc/src/driver/lang.rs`
- `xidlc/src/generate/`
- `xidl-build/src/lib.rs`
- `xidl-rust-axum/README.md`
- `xidlc/doc/idl2rust.md`
- `xidlc/doc/idl2rust-jsonrpc.md`
- `xidlc-examples/api/`
- `xidlc-examples/tests/`

## Coverage before this change

Covered reasonably well:

- basic project overview in `README.md`
- RFC drafts for HTTP, HTTP Stream, HTTP Security, JSON-RPC, and JSON-RPC Stream
- a minimal Axum guide
- a minimal Rust JSON-RPC guide
- a short architecture note
- a short plugin development note

Missing or under-documented:

- complete user path from install to generation
- `xidl-build` usage
- approachable IDL language guide
- `@optional` semantics in user-facing docs
- unified HTTP-family and JSON-RPC-family user docs
- searchable reference pages
- an explicit documentation information architecture
- AI-oriented repository skill guidance

Obsolete or too-thin pages:

- `docs/openapi.md` only listed pragma examples
- `docs/index.md` only included the repository README
- several transport or extension topics existed only as scattered notes

## Maintenance guidance

- Keep user guides practical and example-driven.
- Keep reference docs short, searchable, and cross-linked.
- Keep RFCs formal and normative.
- Prefer linking to implementation-backed notes over duplicating target details
  in multiple places.
