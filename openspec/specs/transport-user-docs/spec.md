# transport-user-docs Specification

## Purpose

TBD - created by archiving change rebuild-documentation-system. Update Purpose
after archive.

## Requirements

### Requirement: HTTP family user documentation is unified and task-oriented

The documentation set SHALL provide one user-facing HTTP guide with dedicated
subsections for HTTP mapping, HTTP Stream, and HTTP Security, written in a
task-oriented style that is simpler than the RFCs.

#### Scenario: User wants to build an HTTP API from IDL

- **WHEN** a reader opens the HTTP guide
- **THEN** the guide explains how to declare routes, parameter sources, body
  behavior, and generated artifacts using practical examples

#### Scenario: User wants to add stream semantics to HTTP

- **WHEN** a reader reads the HTTP guide’s stream subsection
- **THEN** the guide explains server/client stream behavior, codecs, and
  implementation status in user-oriented language

#### Scenario: User wants to secure an HTTP endpoint

- **WHEN** a reader reads the HTTP guide’s security subsection
- **THEN** the guide explains supported security annotations, inheritance, and
  client or server implications with practical examples

### Requirement: JSON-RPC family user documentation is complete

The documentation set SHALL provide user-facing documentation for JSON-RPC,
JSON-RPC Stream, and JSON-RPC security behavior aligned with the repository’s
current generators, runtimes, and RFCs.

#### Scenario: User wants to generate JSON-RPC code

- **WHEN** a reader opens the JSON-RPC guide
- **THEN** the guide explains method naming, params/result shaping, generated
  Rust artifacts, and minimal client/server usage

#### Scenario: User wants to understand JSON-RPC streams

- **WHEN** a reader reads the JSON-RPC stream subsection
- **THEN** the guide explains supported stream modes, framing concepts, and any
  experimental or runtime-specific limitations

#### Scenario: User needs JSON-RPC security expectations

- **WHEN** a reader looks for security guidance in the JSON-RPC docs
- **THEN** the docs explain what security behavior is available today, how it
  relates to transport/runtime concerns, and where the normative definition
  lives
