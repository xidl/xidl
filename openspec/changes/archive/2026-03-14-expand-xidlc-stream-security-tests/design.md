## Context

`xidlc/tests` already drives most code generation coverage through the
folder-based snapshot harness in `xidlc/tests/codegen_snapshot.rs`. That
mechanism is effective for supported fixtures because every `.idl` file under a
known target directory automatically becomes a snapshot case. Stream and HTTP
security coverage, however, is still sparse and uneven:

- stream fixtures currently cover only a few happy-path cases and one
  unsupported bidi-stream example
- security fixtures emphasize simple scheme mapping but do not systematically
  cover inheritance, override, anonymous access, or conflicts
- unsupported combinations that must fail are not represented as a deliberate
  matrix, which makes regressions easy to miss

The requested change is a testing expansion rather than a compiler feature
change. The design therefore needs to maximize coverage without making the
existing snapshot suite harder to maintain.

## Goals / Non-Goals

**Goals:**

- add a systematic stream fixture matrix across `axum`, `openapi`, and `ts`
  targets for both supported and unsupported combinations
- add a systematic HTTP security fixture matrix covering inheritance, override,
  explicit anonymous access, duplicate schemes, and stream-operation
  interactions
- separate positive snapshot coverage from negative validation coverage so
  invalid cases assert on focused error messages instead of snapshot panics
- keep fixture naming and organization explicit enough that future additions can
  fill matrix gaps instead of creating ad hoc one-off files

**Non-Goals:**

- changing stream or security semantics in the compiler or generators
- introducing a new test harness framework beyond what `xidlc` already uses
- exhaustively testing unrelated HTTP features that are already covered by
  existing fixture families

## Decisions

### Use a matrix-oriented fixture set instead of extending a few existing files

New coverage will be expressed as multiple small `.idl` fixtures grouped by
concern, such as supported server-stream variants, invalid stream codec/method
combinations, security inheritance/override, and invalid security annotation
combinations.

This is preferred over appending many cases into `http_stream_sse.idl` or
`http_security.idl` because smaller fixtures isolate snapshot diffs and make it
clear which matrix cell failed. The alternative of using a single “kitchen sink”
IDL file would reduce file count but produce snapshots that are harder to read
and less targeted when one case changes.

### Keep positive cases in target folders and route invalid cases through focused validation tests

Supported cases will continue to live under `xidlc/tests/{axum,openapi,ts}` so
they are automatically exercised by `codegen_snapshot.rs`. Unsupported or
invalid cases will be added as dedicated validation inputs with explicit test
expectations, rather than relying on the snapshot loop to panic.

This preserves the current snapshot ergonomics while improving failure
localization for negative cases. The alternative of teaching the snapshot test
to understand expected failures would overload a simple harness with
per-fixture behavior.

### Mirror cross-target coverage only where target behavior is expected to align

Stream fixtures will be mirrored across `axum`, `openapi`, and `ts` for cases
that all three targets claim to support, such as SSE server streams and NDJSON
client streams. Unsupported combinations will be added only for targets whose
specs require explicit rejection.

This avoids redundant fixtures that all fail for the same reason while still
verifying that each target honors its published support matrix. The alternative
of copying every invalid case into every target directory would create noisy
duplication with little incremental value.

### Encode security coverage as relationship-focused cases

Security fixtures will be organized around effective requirement resolution:
interface-level inheritance, operation-level replacement, `@no-security`
clearing inherited security, duplicate annotations, conflicting combinations,
and the interaction between security annotations and stream HTTP operations.

This structure maps directly to parser, HIR, and generator expectations. The
alternative of grouping only by annotation type would miss override behavior,
which is where most regressions are likely to occur.

### Document the intended matrix in the change artifacts and reflect it in task structure

The design and delta specs will define the required matrix categories, and the
implementation tasks will follow the same breakdown. That gives future work a
clear checklist for adding or updating fixtures.

The alternative of leaving the matrix implicit in filenames would make it too
easy to regress back to ad hoc coverage.

## Risks / Trade-offs

- [Snapshot growth increases review cost] -> Keep fixtures small and focused so
  each snapshot diff remains localized.
- [Negative tests may duplicate generator validation logic across targets] ->
  Reuse shared helpers where possible and prefer one assertion per unsupported
  behavior class.
- [Coverage matrix can drift from actual target support] -> Anchor new fixtures
  to the published stream and security specs, and update the delta specs when
  support rules change.
- [Adding too many near-duplicate fixtures obscures intent] -> Use descriptive
  filenames keyed to behavior, not just transport or target name.

## Migration Plan

This change is test-only and does not require runtime migration. Implementation
will proceed by:

1. auditing existing stream and security fixtures against the intended matrix
2. adding new positive `.idl` fixtures under the relevant target directories
3. adding targeted negative validation tests and any supporting fixture inputs
4. refreshing snapshots and validating the expanded suite

Rollback is straightforward: remove the added fixtures and tests if they prove
too noisy or if the underlying support matrix changes.

## Open Questions

- Whether a small shared helper should be introduced for negative
  code-generation assertions if stream and security invalid cases grow
  substantially during implementation.
- Whether any existing “edge case” fixtures should be split further instead of
  only adding new files beside them.
