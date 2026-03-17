## Context

xidlc-examples currently lacks an HTTP snapshot testing harness. We need a Hurl-like definition format to run HTTP tests, capture full request/response transcripts (headers + body), and store stable snapshots. The workflow must support minijinja variable injection for environment-specific values before execution.

## Goals / Non-Goals

**Goals:**
- Define a Hurl-like HTTP snapshot definition format with multiple requests per file.
- Support minijinja variable injection for runtime configuration before execution.
- Provide a runner that executes requests and records full curl-style transcripts (request + response headers/body).
- Generate one snapshot file per definition file, similar to `codegen_snapshots_from_idl_folders`.
- Add initial HTTP snapshot definitions and outputs for `xidlc-examples/api/http/http_server.idl`.

**Non-Goals:**
- Full Hurl feature parity (assertion DSL, advanced auth flows, complex scripting).
- Replacing existing snapshot systems; this is additive for HTTP examples.
- Supporting non-HTTP protocols.

## Decisions

- **Definition format is Hurl-inspired but minimal.** Keep a small subset of method, path, headers, and body for requests; capture full transcript as output.
  - *Alternatives considered:* Reusing raw curl commands or adopting full Hurl parser. Rejected for complexity and brittleness.
- **Minijinja preprocessing before execution.** Snapshot definition files are templates; variables are injected from test config/env before parsing.
  - *Alternatives considered:* Hardcoded env vars or custom placeholder syntax. Rejected for flexibility and consistency with existing tooling.
- **One file → one snapshot.** Each definition file produces a single snapshot output to simplify review and diffing.
- **Runner modeled after `codegen_snapshots_from_idl_folders`.** Reuse the pattern for discovery, execution, and snapshot writing.

## Risks / Trade-offs

- **[Risk] Snapshot flakiness due to timestamps or dynamic headers** → Mitigation: allow header/body normalization or filtering rules in the runner (e.g., strip Date).
- **[Risk] Environment-dependent variables** → Mitigation: require explicit variable injection and document required variables.
- **[Risk] HTTP server availability during tests** → Mitigation: define a predictable local endpoint and allow base URL injection.

## Migration Plan

- Add definition schema, parser, and runner utilities.
- Add example definitions and expected snapshots for http_server.
- Wire into test suite.
- Document environment variables and usage.
