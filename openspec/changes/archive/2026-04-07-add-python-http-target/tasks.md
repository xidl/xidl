## 1. OpenSpec and Generator Wiring

- [x] 1.1 Add `python` and `python-http` to the generator registry, driver
      language parsing, and codegen dispatch paths
- [x] 1.2 Create the `xidlc/src/generate/python/` and
      `xidlc/src/generate/python_http/` module structure with `minijinja`-based
      renderers and templates
- [x] 1.3 Add initial Python target properties and artifact output rules
      consistent with other language targets

## 2. Python Runtime

- [x] 2.1 Create the top-level `python/` runtime package layout for shared
      generated-code support
- [x] 2.2 Implement framework-neutral request, response, error, auth, and stream
      helpers used by generated `python-http` code
- [x] 2.3 Define the adapter-facing registration surface needed for Django and
      FastAPI integration without coupling the core runtime to either framework

## 3. Plain Python Generator

- [x] 3.1 Implement plain `python` model generation for core declarations used
      by generated Python code
- [x] 3.2 Render Python modules and imports through `minijinja` templates with
      stable output suitable for snapshot testing
- [x] 3.3 Ensure non-HTTP generated Python artifacts can be emitted alongside
      HTTP-specific output without runtime leakage

## 4. Python HTTP Generator

- [x] 4.1 Reuse shared HTTP/security/stream normalization to build Python HTTP
      method metadata and validation
- [x] 4.2 Generate abstract Python service interfaces for HTTP operations and
      structured metadata for routes, media types, and security requirements
- [x] 4.3 Generate automatic async route-registration helpers that preserve the
      supported unary, security, and stream RFC semantics for server
      integrations; defer client generation

## 5. Test Coverage

- [x] 5.1 Add `xidlc/tests/python/` and `xidlc/tests/python-http/` snapshot
      fixtures covering plain Python generation plus HTTP, security, and stream
      behavior
- [x] 5.2 Extend `xidlc/tests/codegen_snapshot.rs` and
      `xidlc/tests/http_validation.rs` to include Python and Python HTTP
      coverage
- [x] 5.3 Add Python integration tests and a `Makefile` modeled on
      `golang/Makefile`, including representative Django and FastAPI adapter
      flows

## 6. Verification

- [x] 6.1 Run the relevant Rust snapshot and validation tests for the new
      generators
- [x] 6.2 Run the Python integration test workflow and update expected generated
      outputs if needed
- [x] 6.3 Fix regressions across shared HTTP behavior exposed by the new Python
      target
