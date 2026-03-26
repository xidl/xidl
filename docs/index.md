# XIDL Documentation

XIDL is an OMG IDL-based toolchain for generating APIs, runtimes, and schema
artifacts from a single IDL source. The documentation is organized by audience
so you can move directly to the material you need.

## Choose a path

### User Guides

Use the user guides when you want to install `xidlc`, write IDL, generate code,
or understand the supported HTTP and JSON-RPC workflows.

- Start with [Installation](user/install.md)
- Continue with [Using xidlc](user/xidlc.md)
- Learn the language in [IDL Guide](user/idl.md)
- Explore transports in [HTTP Guide](user/http.md) and
  [JSON-RPC Guide](user/jsonrpc.md)

### Reference

Use the reference section when you need a fast answer about syntax, annotations,
pragmas, or target support.

- [Reference Overview](reference/index.md)
- [IDL Elements](reference/idl-elements.md)
- [Annotations](reference/annotations.md)
- [Pragmas](reference/pragmas.md)
- [Targets](reference/targets.md)

### RFCs

Use the RFCs when you need the formal transport mapping rules. User guides link
to them for exact semantics.

- HTTP, HTTP Stream, and HTTP Security
- JSON-RPC and JSON-RPC Stream

### Development

Use the development section when you want to understand the repository
architecture, code generation pipeline, or plugin model.

- [Development Overview](development/index.md)
- [Architecture](architecture.md)
- [Plugin Development](plugin.md)
- [Documentation Audit](development/doc-audit.md)

### AI Skills

Use the AI section when you want a compact, operational summary for an agent or
automation workflow.

- [XIDL AI Skill](ai/index.md)

## Source of truth

This documentation is implementation-backed. When a behavior matters, the most
authoritative sources are:

- the generator code under `xidlc/src/generate/`
- the transport RFCs under `docs/rfc/`
- runnable examples under `xidlc-examples/`
- build integration code under `xidl-build/`

If a page looks too high-level for your task, jump from the guide to the linked
reference or RFC page.
