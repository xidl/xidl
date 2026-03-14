## Context

The current Rust generation path has bespoke support for a narrow set of
annotations such as `@derive(...)` and field-name mapping, but it does not have
an escape hatch for arbitrary Rust attributes. That forces users to wait for
new compiler features whenever a Rust crate expects metadata such as
`#[serde(rename = "...")]`, `#[serde(default)]`, or similar attributes.

This change is smaller than a parser redesign because the annotation grammar and
HIR already preserve raw annotation parameters. The main work is deciding where
Rust passthrough attributes attach in generated output and ensuring only Rust
backends observe them.

## Goals / Non-Goals

**Goals:**
- Add `@rust(...)` as a target-specific passthrough for generated Rust
  attributes.
- Reuse the existing annotation representation so the compiler preserves the
  attribute body verbatim instead of normalizing Rust syntax.
- Emit passthrough attributes on generated Rust declarations and fields where
  the source annotation is attached.
- Add focused snapshots and docs that demonstrate serde-oriented usage.

**Non-Goals:**
- Validate arbitrary Rust attribute syntax beyond existing IDL annotation
  parsing.
- Introduce equivalent passthrough behavior for C, C++, TypeScript, OpenAPI, or
  other generators.
- Replace existing convenience annotations such as `@derive(...)`.
- Guarantee that every possible annotation placement is meaningful to the Rust
  compiler; this change only preserves and emits what the IDL author wrote.

## Decisions

### 1. Model `@rust(...)` as a generator-only annotation convention

Decision:
- Keep `@rust(...)` in the existing generic annotation model and interpret it
  only inside Rust code generators.

Rationale:
- The parser and HIR already capture annotation names and raw parameters.
- This avoids broad schema or AST changes for behavior that is explicitly
  target-specific.

Alternatives considered:
- Add a first-class typed HIR node for Rust passthrough attributes.
  Rejected because it would add compiler surface area without improving codegen
  fidelity.

### 2. Preserve the annotation body verbatim

Decision:
- Emit `@rust(x)` as `#[x]` using the raw body text exactly as authored, after
  trimming only the outer annotation wrapper.

Rationale:
- Rust attributes frequently contain nested paths, quoted strings, and token
  patterns that do not fit the normalized key/value helpers used by simpler
  annotations.
- Verbatim passthrough is the smallest feature that supports real-world cases
  like `serde(rename = "camelCase")`.

Alternatives considered:
- Parse `@rust(...)` into normalized key/value pairs.
  Rejected because it would break many valid Rust attribute forms and create a
  second attribute grammar to maintain.

### 3. Attach passthrough attributes at the same semantic emission site as the source annotation

Decision:
- Emit item-level `@rust(...)` annotations immediately above the generated Rust
  item and field-level annotations immediately above the generated Rust field.

Rationale:
- This matches user intent and keeps the mapping predictable.
- It also aligns with the current `@derive(...)` behavior, which is already
  emitted on generated Rust items.

Alternatives considered:
- Restrict passthrough to top-level declarations only.
  Rejected because the motivating serde use cases often need field attributes.

### 4. Preserve multiple passthrough annotations in source order

Decision:
- Collect every `@rust(...)` annotation on a node and emit one Rust `#[...]`
  line per annotation in the original order.

Rationale:
- Rust commonly stacks attributes, and ordering can matter for readability or
  macro behavior.
- Treating them as a list avoids inventing a merge policy.

Alternatives considered:
- Merge multiple annotations into a single composite attribute.
  Rejected because there is no safe generic merge rule.

## Risks / Trade-offs

- [Risk] Verbatim passthrough can emit invalid Rust if the attribute body is
  malformed. -> Mitigation: document that `@rust(...)` is intentionally
  unchecked and add snapshots covering representative valid forms.
- [Risk] Different Rust backends may not all share the same rendering helpers.
  -> Mitigation: centralize extraction in Rust utility code and thread the
  result through each backend that emits Rust items.
- [Risk] Attribute placement could drift from user expectations for constructs
  that expand into multiple generated items. -> Mitigation: scope the first pass
  to direct declaration and field emission sites already covered by snapshots.

## Migration Plan

1. Add shared Rust utility helpers that extract ordered `@rust(...)`
   passthrough attributes from annotations.
2. Update Rust generators and templates to emit those attributes on supported
   items and fields alongside existing derives and docs.
3. Add or update snapshot fixtures and Rust generator documentation to show the
   new annotation.
4. Verify non-Rust snapshots remain unchanged.

Rollback strategy:
- Revert the generator changes and fixture updates together if emitted Rust
  output proves too permissive or incorrectly placed.

## Open Questions

- Should the first implementation cover every Rust-emitting backend
  (`rust`, `rust-axum`, `rust-jsonrpc`) immediately, or start with the base
  Rust generator and extend if shared code paths make the broader scope cheap?
- Are there declaration kinds whose generated output fans out into multiple Rust
  items and therefore need an explicit placement rule beyond “attach to the
  emitted item”?
