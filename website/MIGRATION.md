# Documentation Structure

This file records where each piece of the documentation lives and how the
current structure was reached.

## Final Structure

```
src/content/docs/
├── index.mdx            # Home (splash)
├── changelog.md         # Generated from CHANGELOG.md by scripts/copy-changelog.mjs
├── guide/               # Onboarding and happy path (Rust + HTTP)
│   ├── index.mdx        # Install & Quickstart
│   ├── first-http-api.mdx
│   ├── rust-integration.mdx
│   ├── editor.mdx
│   └── language-support.mdx
├── docs/                # Language and tooling reference
│   ├── index.mdx        # Language Basics
│   ├── xidlc.mdx        # xidlc CLI
│   ├── targets.mdx      # Target Generators
│   ├── annotations.mdx  # Built-in annotations
│   └── pragmas.mdx      # Compiler pragmas
├── rest/                # HTTP & REST mapping
│   ├── index.mdx        # HTTP & REST Basics
│   ├── serialize.mdx
│   ├── stream.mdx
│   ├── security.mdx
│   ├── rust-axum.mdx
│   ├── typescript-server.mdx
│   └── openapi.mdx
├── jsonrpc/             # JSON-RPC mapping
│   ├── index.mdx
│   └── rust-jsonrpc.mdx
├── rfc/                 # Normative specifications
│   ├── index.mdx
│   ├── http.mdx
│   ├── http-security.mdx
│   ├── http-stream.mdx
│   ├── jsonrpc.mdx
│   └── jsonrpc-stream.mdx
└── ai/                  # Agent-facing summary
    └── index.mdx
```

## Migration Notes

- **Guide** focuses on onboarding and the "Happy Path" (Rust + HTTP).
- **Reference** (`docs/`) is the language and tooling reference.
- **HTTP & REST** (`rest/`) and **JSON-RPC** (`jsonrpc/`) cover protocol
  mapping.
- **RFC** remains normative and stable.
- Duplicate pages were merged: `quickstart.mdx` into `guide/index.mdx`,
  `rest/pragma.mdx` into `docs/pragmas.mdx`, `rest/nextjs.mdx` into
  `rest/typescript-server.mdx`, and `builtin_annotation.mdx` became
  `docs/annotations.mdx`.
- Sidebar group labels and page titles use normal words, not target/variable
  names (for example `HTTP & REST`, `Rust Axum Target`).
