## Context

`go-http` already has the basic structure we want: shared HTTP annotation
parsing, generated `net/http` handlers and clients, a small Go runtime, Rust
snapshot tests, and Go example integration tests. The problem is coverage and
contract drift. The published RFC set spans unary bindings, media types,
security inheritance, API-key variants, OAuth2 metadata, stream bindings, and
validation behavior that the current Go target either does not implement yet or
does not prove through dedicated fixtures.

The repository has already converged on a pattern that works for other targets:

- shared normalization and validation in `xidlc/src/generate/utils/http.rs`
- target-specific rendering in `xidlc/src/generate/<target>/`
- focused snapshot fixtures under `xidlc/tests/<target>/`
- end-to-end generated-code tests in `golang/xidlc-examples`

This change should extend that pattern instead of inventing Go-only RFC logic.

## Goals / Non-Goals

**Goals:**

- Close the behavior gap between the published HTTP RFC documents and the
  `go-http` target.
- Reuse shared HTTP/security/stream normalization wherever the RFC semantics are
  target-independent.
- Add explicit Go HTTP snapshot, validation, and example coverage for the RFC
  matrix instead of depending on two broad fixtures.
- Model HTTP security in generated Go code with enough structure to preserve
  basic, bearer, API key, and OAuth2 requirement metadata.

**Non-Goals:**

- Add bidirectional HTTP streaming.
- Introduce a framework-specific Go adapter.
- Redesign the Go runtime around middleware or interceptor abstractions.
- Solve OpenAPI-specific behavior beyond what is needed to stay aligned with the
  HTTP RFC contracts.

## Decisions

### 1. Mirror the RFC matrix into focused `golang-http` fixtures

Decision:

- Add dedicated `xidlc/tests/golang-http/*.idl` fixtures for the unary,
  security, and stream behavior classes that already exist for `axum`, `ts`, and
  `openapi`.
- Keep each fixture narrow: bindings/defaults, response-shape, security
  inheritance, stream bindings, and invalid cases remain separate.

Rationale:

- The current `go-http` fixture set is too small to tell which RFC behavior is
  broken when snapshots change.
- Focused fixtures align with the existing snapshot discipline in the repo and
  make RFC coverage auditable.

Alternatives considered:

- Expanding the two existing Go HTTP fixtures into catch-all files. Rejected
  because that would keep failures hard to localize.

### 2. Keep RFC semantics in shared HTTP utilities and reserve Go code for projection details

Decision:

- Any missing route normalization, parameter-source resolution, effective
  security computation, or stream validation should be fixed in shared HTTP
  utilities first when the rule is not Go-specific.
- `go-http` should only own projection choices: generated type shape, runtime
  helper APIs, `net/http` request/response wiring, and Go-specific metadata
  emission.

Rationale:

- `axum`, `openapi`, `ts`, and `go-http` should not drift on the same RFC rule.
- Shared fixes reduce the chance that new Go coverage exposes a latent
  cross-target bug and then leaves the targets inconsistent.

Alternatives considered:

- Patching `go-http` only. Rejected because it would duplicate RFC logic that is
  already centralized elsewhere.

### 3. Extend Go security metadata instead of hard-coding only the currently tested schemes

Decision:

- Expand `golang/xidl-go-http` security metadata so generated
  `...SecurityRequirements()` helpers can represent every RFC-declared scheme
  the target supports, including OAuth2 scope metadata.
- Keep runtime enforcement minimal and transport-level:
  - basic and bearer continue using `Authorization`
  - API key continues using header/query/cookie
  - OAuth2 is modeled as bearer-style credential transport plus declared scopes
    in metadata

Rationale:

- The RFC already treats OAuth2 as a scope-carrying HTTP security requirement.
- Go clients and handlers need a structured requirement model even if actual
  authorization remains application-owned.

Alternatives considered:

- Ignoring OAuth2 in generated Go metadata. Rejected because it would make the
  Go target knowingly incomplete relative to `docs/rfc/http-security.md`.

### 4. Prove behavior in three layers: snapshot, validation, and end-to-end examples

Decision:

- Use snapshots to verify generated structure.
- Use Rust-side invalid-fixture tests to verify rejected combinations.
- Use Go example tests to verify runtime interoperability, auth propagation,
  status handling, and stream framing through real HTTP interaction.

Rationale:

- Snapshot tests alone do not prove request/response behavior.
- Go example tests alone do not localize generator regressions well.
- Validation tests protect the explicit rejection rules in the RFCs.

Alternatives considered:

- Relying primarily on Go integration tests. Rejected because they are slower
  and do not cover generation-only regressions precisely.

### 5. Keep example coverage aligned to representative RFC flows, not every fixture

Decision:

- `golang/xidlc-examples` should add a few representative APIs that exercise the
  hardest semantics together:
  - unary bindings and media types
  - security inheritance/override and multiple scheme locations
  - server-stream and client-stream flows with auth and final unary responses
- It should not mirror every snapshot fixture one-for-one.

Rationale:

- Example tests should protect generated runtime behavior, not duplicate the
  entire snapshot suite.
- A smaller set of representative end-to-end cases keeps Go tests maintainable.

Alternatives considered:

- Creating one Go example per fixture. Rejected because it would duplicate the
  snapshot matrix and slow test maintenance.

## Risks / Trade-offs

- `OAuth2 metadata expands runtime APIs` → Keep the runtime surface additive and
  transport-focused so existing basic/bearer/api-key users do not break.
- `More Go fixtures increase snapshot churn` → Use narrow fixtures with stable
  names so failures identify one RFC concern at a time.
- `Shared HTTP fixes may change other targets` → Run the existing cross-target
  snapshot and validation suites when implementation starts.
- `Go example tests can become too broad` → Limit them to representative
  integration flows and leave exhaustive shape coverage to snapshots.
