# XIDL

![demo](./assets/demo.svg)

Define interfaces once. Generate APIs, SDKs, specs, and tooling from one
source of truth.

XIDL is an IDL-first contract platform for teams that want one interface
definition to drive HTTP, JSON-RPC, streaming APIs, security metadata,
generated SDKs, and machine-readable specs. It combines contract authoring,
protocol mapping, code generation, and interactive tooling into one workflow.

![Crates.io Version](<https://img.shields.io/crates/v/xidlc?label=xidlc(crates.io)>)
![Crates.io Version](<https://img.shields.io/crates/v/xidl-build?label=xidl-build(crates.io)>)
![GitHub Release](https://img.shields.io/github/v/release/xidl/xidl)
[![VS Code Extension](https://img.shields.io/badge/Install-VS%20Code%20Extension-007ACC?logo=visualstudiocode&logoColor=white)](https://marketplace.visualstudio.com/items?itemName=cathaysia.vscode-idl-language)
[![GitHub](https://img.shields.io/badge/View_on-GitHub-181717?logo=github)](https://github.com/xidl/xidl)

## What XIDL Makes Possible

![XIDL capabilities](website/public/assets/xidl-capabilities.svg)

XIDL works well as the contract layer for API teams because interface intent is
explicit, structured, and centralized. That makes the system easier for humans
to review and easier for tools and AI systems to understand, generate from,
lint, and keep in sync.

## Why Teams Use XIDL

- One contract drives multiple protocols and outputs.
- HTTP and JSON-RPC live in the same interface system instead of separate toolchains.
- Streaming and security annotations stay attached to the contract, not scattered across framework code.
- Specs, SDKs, stubs, examples, and tests can all be generated from the same IDL.
- Structured contracts are easier for automation, agents, and interactive tools to reason about.

## Core Capabilities

- Interface-first development with OMG IDL-compatible foundations and XIDL extensions.
- Protocol mappings for HTTP and JSON-RPC, plus stream-oriented workflows.
- Generated outputs for Rust, TypeScript, C, C++, OpenAPI, and OpenRPC.
- Security-aware contracts including HTTP auth and API key mappings.
- Formatting support through `xidlc fmt`.
- Editor and language tooling through [`idl-language-server`](https://github.com/xidl/idl-language-server).
- Interactive HTTP exploration from IDL, including launching client workflows with tools such as Scalar.

## What One IDL Produces

![XIDL generated outputs](website/public/assets/xidl-outputs.svg)

One XIDL contract can radiate into runtime surfaces, generated SDKs, machine
readable specs, and implementation assets that stay aligned because they come
from the same source.

## What You Can Build

- HTTP services and generated clients from one IDL contract.
- JSON-RPC services and generated clients from the same modeling approach.
- OpenAPI and OpenRPC specs that stay aligned with implementation contracts.
- Stream-oriented APIs with shared contract semantics.
- Contract-driven examples, tests, and review flows.

## How Teams Use XIDL

![XIDL workflow](website/public/assets/xidl-workflow.svg)

Teams author and review the contract once, then generate the artifacts they
need to build servers, ship clients, publish specs, and keep examples and tests
synchronized.

## Quick Start

Install `xidlc`:

```bash
cargo install xidlc
```

Format IDL files:

```bash
xidlc fmt --inplace api.idl
```

Use this repository as a `pre-commit` hook:

```yaml
repos:
  - repo: https://github.com/xidl/xidl
    rev: v0.31.0
    hooks:
      - id: xidlc-fmt
```

Generate Rust types:

```bash
xidlc gen --out-dir out rust api.idl
```

Generate an Axum HTTP surface:

```bash
xidlc gen --out-dir out rust-axum api.idl
```

Generate a TypeScript REST client (the default mode):

```bash
xidlc gen --out-dir out typescript-rest api.idl
```

Generate TypeScript REST client and server contracts:

```bash
xidlc gen --out-dir out typescript-rest --client --server api.idl
```

Generate OpenAPI:

```bash
xidlc gen --out-dir generated openapi api.idl
```

## Documentation

- [Install & Quickstart](website/src/content/docs/guide/index.mdx)
- [Your First HTTP API](website/src/content/docs/guide/first-http-api.mdx)
- [Using xidlc](website/src/content/docs/docs/xidlc.mdx)
- [HTTP & REST Basics](website/src/content/docs/rest/index.mdx)
- [TypeScript REST Server](website/src/content/docs/rest/typescript-server.mdx)
- [JSON-RPC Basics](website/src/content/docs/jsonrpc/index.mdx)
- [Targets Reference](website/src/content/docs/docs/targets.mdx)
- [HTTP RFC](website/src/content/docs/rfc/http.mdx)
- [JSON-RPC RFC](website/src/content/docs/rfc/jsonrpc.mdx)

## Links

- [Linux Do](https://linux.do/)
- [XIDL Language Server](https://github.com/xidl/idl-language-server)
