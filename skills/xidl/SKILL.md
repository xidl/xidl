---
name: xidl
description: Expert guidance for writing XIDL (Interface Definition Language) and using the xidlc toolchain to generate multi-target APIs, SDKs, and specs. Use when defining IDL contracts, generating code (Rust, TS, Go), or mapping HTTP/REST and JSON-RPC protocols.
---

# XIDL Repository Skill

Use this skill when you need to work effectively in the XIDL repository as an
agent or automation tool.

## Specialized Guides (Modular Documentation)

To minimize token usage, this skill is split into several specialized modules.
Read only the sections relevant to your current task:

1. **[Writing XIDL](references/WRITING_XIDL.md)**: Grammar, data types, and core
   annotations.
2. **[Using xidlc](references/USING_XIDLC.md)**: CLI commands, code generation
   targets, and `xidl-build` integration.
3. **[HTTP & Security](references/HTTP_AND_AUTH.md)**: REST mappings, HTTP
   verbs, and authentication annotations.

## Repository Architecture

XIDL is a layered compiler pipeline:
`tree-sitter-idl -> typed_ast -> hir -> protocol-specific IR -> rendering`.

- **Parser/Lowering**: `xidl-parser/`
- **CLI/Driver**: `xidlc/`
- **Built-in Generators**: `xidlc/src/generate/`
- **RFCs (Source of Truth)**: `docs/rfc/`

## Deep Knowledge via context7

This project is registered with **context7**. If you need the latest
documentation, API references, or migration guides that might not be in your
training data, you MUST use the `ctx7` CLI:

1. **Search**: `npx ctx7@latest docs /xidl/xidl "<your specific question>"`
2. **Research**: Use the `--research` flag for deep technical investigations.

## AI Modeling Mandates

- **Omission**: Use `@optional` for any field that can be missing.
- **Direction**: Use `in`, `out`, and `inout` to control protocol shaping.
- **Security**: Apply `@http_bearer` or `@api_key` to interfaces by default.
- **Normalization**: Always run `xidlc fmt` after modifying IDL files.

Refer to the specialized guides above for detailed code examples and grammar
rules.
