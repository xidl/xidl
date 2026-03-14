## Context

The repository already contains three distinct testing surfaces for HTTP-related
behavior:

- `xidlc/tests`, which validates generator output through IDL fixtures,
  snapshots, and focused negative cases
- generator unit tests, which catch target-specific validation and panic paths
- `xidlc-examples/tests`, which exercise generated clients and servers through
  real HTTP or JSON-RPC flows

Coverage for `http`, `http-security`, and `http-stream` exists, but it is not
systematic. Unary HTTP mapping is stronger in `axum` and `openapi` than in
`ts`, HTTP security is missing some invalid and target-spanning cases, and
stream coverage is better than before but still leaves unsupported combinations
and example-level regressions exposed. The user requirement is to make coverage
comprehensive against the three published specs, not just to add a few more
happy-path fixtures.

## Goals / Non-Goals

**Goals:**

- define an explicit coverage matrix for `http`, `http-security`, and
  `http-stream` so each spec requirement has a corresponding generator-level or
  integration-level test
- expand `xidlc/tests` with both positive snapshots and negative validation
  cases across `axum`, `openapi`, and `ts` where those targets are relevant
- expand `xidlc-examples/tests` so generated example services demonstrate
  end-to-end behavior for the same spec-backed concerns
- keep the test layout understandable enough that future RFC or generator
  changes can identify missing coverage quickly

**Non-Goals:**

- changing the semantics of the HTTP, security, or stream generators
- replacing the existing snapshot or example test harnesses with a new test
  framework
- adding exhaustive coverage for non-HTTP capabilities that are unrelated to
  the three target specs

## Decisions

### Use a two-tier coverage model

Generator-level behavior will be validated in `xidlc/tests`, while runtime and
interoperability behavior will be validated in `xidlc-examples/tests`.

This is preferred over forcing everything into `xidlc/tests` because snapshot
fixtures are good at checking generated shape and validation errors, but they do
not prove that generated clients and servers interoperate correctly. The
alternative of relying only on example tests would leave target-specific
negative paths under-covered.

### Organize test additions by spec concern, not by target alone

Each new fixture or integration test will be tied to a concrete spec concern:
binding behavior, defaults, inheritance, explicit anonymous access, stream
directionality, unsupported method/codec combinations, and so on.

This is preferred over simply cloning more target-specific fixtures because the
user requirement is full coverage of the published specs. The alternative would
create more files without making it obvious which spec gap each file closes.

### Keep invalid coverage focused and target-aware

Negative cases in `xidlc/tests` will continue to use targeted assertions
instead of snapshot failures. Generator targets that return normal errors and
targets such as OpenAPI that currently panic on invalid cases will each use the
most direct assertion surface already present in the codebase.

This avoids overfitting a single harness to incompatible failure modes. The
alternative of forcing every target through one generic invalid-IDL runner would
either hide panic-only behavior or make tests brittle.

### Extend examples only where they prove additional value

`xidlc-examples` will be extended for behaviors that benefit from real client /
server interaction, such as unary HTTP request bindings, security-sensitive
generated output wiring, and streamed request/response flows.

This is preferred over mirroring every `xidlc/tests` fixture as an example.
Examples are slower and more stateful, so they should validate integration
behavior that snapshots cannot cover well.

### Update coverage artifacts in lockstep with test additions

The change artifacts will define the expected matrix categories, and the task
list will mirror those categories. That gives future contributors a checklist to
maintain as the three specs evolve.

The alternative of treating this as an implementation-only test expansion would
risk drifting back into ad hoc additions.

## Risks / Trade-offs

- [Coverage expansion creates too many near-duplicate fixtures] -> Name fixtures
  by behavior class and avoid target copies when a case is irrelevant to that
  target.
- [Target-specific invalid behavior differs between `Err` and `panic`] ->
  Assert invalid paths at the most stable layer for each target and keep helper
  code small and explicit.
- [Example tests become noisy or flaky] -> Prefer deterministic in-process or
  ephemeral-port setups that already exist in `xidlc-examples/tests`.
- [Spec coverage claims become stale] -> Tie each requirement to concrete
  fixture or integration categories so missing coverage is easy to detect during
  future changes.

## Migration Plan

This is a test-only change and requires no runtime migration. Implementation
will proceed by:

1. auditing the three specs against existing `xidlc/tests` and
   `xidlc-examples/tests`
2. expanding `xidlc/tests` for unary HTTP, security, and stream generator
   coverage
3. expanding `xidlc-examples/tests` for end-to-end HTTP and stream validation
4. running the relevant package test suites and refreshing snapshots where
   needed

Rollback is straightforward: remove the added fixtures, snapshots, and example
tests if they prove redundant or if the spec boundaries are redefined.

## Open Questions

- Whether HTTP security example coverage should assert generated document shape,
  runtime transport behavior, or both for the highest-value cases.
- Whether some existing unary HTTP fixtures should be split further before more
  matrix cases are added beside them.
