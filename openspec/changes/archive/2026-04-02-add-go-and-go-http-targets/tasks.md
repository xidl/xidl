## 1. Shared Design and Target Plumbing

- [x] 1.1 Register `go` and `go-http` as recognized `xidlc` generator targets.
- [x] 1.2 Add generator module scaffolding under `xidlc/src/generate/go/` and
      `xidlc/src/generate/go_http/`.
- [x] 1.3 Reuse existing shared HTTP/security/stream validation utilities so Go
      targets consume the same normalized contract as `rust-axum`.

## 2. `go` Language Projection

- [x] 2.1 Implement plain Go type generation for structs, enums, aliases,
      constants, and request/response helper types needed by interfaces.
- [x] 2.2 Add `xidlc/tests/go/` snapshot fixtures covering the language-level
      projection.
- [x] 2.3 Add a minimal `xidl-go` runtime/helper package only if generated Go
      output requires shared code.

## 3. `go-http` Unary Runtime and Generation

- [x] 3.1 Create a new workspace member `xidl-go-http` built on standard-library
      `net/http`.
- [x] 3.2 Implement generated unary server registration and client code aligned
      with `docs/rfc/http.md`.
- [x] 3.3 Implement request/response media-type handling for JSON, form, and
      msgpack where supported.
- [x] 3.4 Add or refresh snapshot coverage for unary Go HTTP projection.

## 4. HTTP Security Alignment

- [x] 4.1 Implement generated server-side auth extraction for basic, bearer, and
      API key schemes.
- [x] 4.2 Implement generated client-side auth application for the same scheme
      set.
- [x] 4.3 Add Go HTTP snapshot and integration coverage for security
      inheritance, overrides, and `@no_security`.

## 5. HTTP Stream Alignment

- [x] 5.1 Implement server-stream SSE support in `xidl-go-http` and generated
      `go-http` output.
- [x] 5.2 Implement client-stream NDJSON support in `xidl-go-http` and generated
      `go-http` output.
- [x] 5.3 Add snapshot and example coverage for supported Go HTTP stream
      methods.

## 6. Examples and Integration

- [x] 6.1 Create a new workspace member `xidlc-examples-go` with its own
      `go.mod`, generated HTTP fixtures, and test layout.
- [x] 6.2 Mirror the key unary/security/stream examples from `xidlc-examples`
      into `xidlc-examples-go`.
- [x] 6.3 Add repository test commands and CI coverage for Go example tests.

## 7. Optional Adapter Follow-up

- [x] 7.1 Keep a documented seam for framework adapters such as `gin` without
      making them part of the initial core runtime.
- [x] 7.2 If needed, prototype a build-tagged or separate-package adapter after
      the framework-neutral runtime is stable.
