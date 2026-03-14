## Why

`xidlc` already supports a few hard-coded Rust annotation conveniences such as
`@derive(...)`, but that does not cover the wider Rust attribute surface needed
by downstream crates. Adding `@rust(...)` lets IDL authors pass through
target-specific attributes such as `#[serde(rename = "camelCase")]` without
teaching the compiler every Rust ecosystem convention.

## What Changes

- Add a Rust-targeted `@rust(...)` annotation that emits `#[...]` into generated
  Rust code using the annotation body verbatim.
- Apply `@rust(...)` passthrough on generated Rust items and fields for the
  Rust backends that already materialize those annotated declarations.
- Preserve multiple `@rust(...)` annotations in source order so stacked Rust
  attributes remain expressible.
- Add snapshot coverage and documentation for Rust attribute passthrough,
  including `serde`-style field attributes.

## Capabilities

### New Capabilities
- `rust-annotation-passthrough`: Defines how `@rust(...)` annotations map to
  raw Rust `#[...]` attributes in generated Rust code.

### Modified Capabilities
- None.

## Impact

- Affected code: Rust generator annotation utilities, Rust codegen templates /
  renderers, and Rust snapshot fixtures under `xidlc/tests`.
- Affected APIs: accepted IDL annotation syntax for Rust generation and the
  generated Rust source seen by downstream consumers.
- Risk areas: preserving raw attribute text exactly, attaching attributes at the
  correct emitted Rust item or field, and keeping non-Rust generators
  unchanged.
