## 1. Shared HTTP and Go HTTP Plumbing

- [x] 1.1 Audit the RFC behaviors in `docs/rfc/http*.md` against the current
      `go-http` generator and runtime to identify the remaining implementation
      gaps.
- [x] 1.2 Move any missing route, binding, security, or stream normalization
      rules into shared HTTP utilities when they are not Go-specific.
- [x] 1.3 Extend Go HTTP metadata/runtime types so generated code can represent
      the full supported security requirement set, including OAuth2 scopes if
      needed.

## 2. Unary HTTP RFC Alignment

- [x] 2.1 Implement any missing `go-http` unary binding behavior for path,
      query, header, cookie, body, defaults, and route-template handling.
- [x] 2.2 Implement any missing Go HTTP response semantics for default success
      status, response shape, and deprecation metadata propagation.
- [x] 2.3 Add focused `xidlc/tests/golang-http/` unary fixtures that cover the
      RFC binding and media-type categories currently missing from Go HTTP
      snapshots.

## 3. HTTP Security RFC Alignment

- [x] 3.1 Implement any missing Go HTTP handling for security inheritance,
      operation-level replacement, and `@no_security` override behavior.
- [x] 3.2 Implement any missing Go HTTP metadata or runtime support for
      supported security schemes, including API key locations and OAuth2
      scope-carrying requirements.
- [x] 3.3 Add focused Go HTTP security fixtures and validation assertions for
      duplicated, conflicting, and malformed security annotations.

## 4. HTTP Stream RFC Alignment

- [x] 4.1 Implement any missing Go HTTP stream behavior for supported
      server-stream and client-stream flows, including binding preservation and
      final unary responses.
- [x] 4.2 Add focused Go HTTP stream fixtures that cover supported directions,
      request bindings, and invalid stream combinations.
- [x] 4.3 Verify shared stream validation continues to reject unsupported Go
      HTTP combinations such as invalid codecs, methods, and shapes.

## 5. Example and Regression Coverage

- [x] 5.1 Expand `golang/xidlc-examples` with representative unary RFC flows
      that prove generated request binding, status handling, and response
      decoding end to end.
- [x] 5.2 Expand `golang/xidlc-examples` with representative security-aware and
      stream-aware flows that prove auth propagation, SSE/NDJSON behavior, and
      final unary responses.
- [x] 5.3 Update Rust-side snapshot and validation tests plus Go example test
      execution so the completed Go HTTP RFC matrix is enforced in CI.
