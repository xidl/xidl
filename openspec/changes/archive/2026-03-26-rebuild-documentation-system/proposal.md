## Why

The current documentation is fragmented across README files, RFC drafts, target
notes, and a few contributor guides, which makes it hard for users to learn the
IDL, hard for contributors to understand the architecture, and impossible for AI
tooling to consume a single authoritative skill. We need a coherent
documentation system now because XIDL already exposes multiple transports,
targets, and extension points, but the docs do not yet provide a complete path
from installation to production use and plugin development.

## What Changes

- Reorganize the docs site into clear user, reference, RFC, development, and AI
  skill sections with navigation that matches those audiences.
- Add complete user documentation for `xidlc` installation and usage,
  `xidl-build` usage in Rust, IDL syntax and behavior, and target-oriented
  generation workflows.
- Add transport user documentation for HTTP, HTTP Stream, HTTP Security,
  JSON-RPC, JSON-RPC Stream, and JSON-RPC security semantics, with user-focused
  explanations distinct from the RFCs.
- Add reference documentation that covers all supported IDL elements,
  annotations, pragmas, targets, and transport-specific constructs in a
  searchable format.
- Expand development documentation so contributors can understand repository
  architecture, code generation flow, extension points, and plugin development.
- Add an AI-facing skill document that teaches an agent how to work with XIDL’s
  syntax, generators, docs, examples, and plugin model.

## Capabilities

### New Capabilities

- `documentation-navigation`: The documentation site presents a complete,
  audience-oriented information architecture for user docs, reference docs,
  RFCs, development docs, and AI skills.
- `idl-user-reference-docs`: The documentation set teaches XIDL installation,
  `xidlc`, `xidl-build`, IDL syntax, annotations, behavior, and target
  generation, and provides full searchable reference coverage.
- `transport-user-docs`: The documentation set explains HTTP, HTTP Stream, HTTP
  Security, JSON-RPC, JSON-RPC Stream, and related security behavior in
  user-oriented guides that align with the implemented generators and RFCs.
- `contributor-and-ai-docs`: The repository includes complete contributor docs
  for architecture and plugin development, plus an AI skill document that
  encodes XIDL usage and contribution workflows.

### Modified Capabilities

- (none)

## Impact

- `mkdocs.yml` navigation and the `docs/` information architecture.
- Existing user-facing docs such as `README.md`, `docs/openapi.md`,
  `docs/rust-axum.md`, `docs/rust-jsonrpc.md`, `docs/pragma.md`,
  `docs/xidl-extend.md`, and RFC pages.
- Contributor docs including `docs/architecture.md` and `docs/plugin.md`.
- New documentation content covering `xidlc`, `xidl-build`, IDL syntax,
  transport guides, and reference material.
- A new AI-facing skill artifact and any supporting published docs pages.
