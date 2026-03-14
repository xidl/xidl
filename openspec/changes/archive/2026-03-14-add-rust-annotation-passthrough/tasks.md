## 1. Rust Annotation Extraction

- [x] 1.1 Add shared Rust generator helpers that collect ordered `@rust(...)` annotation bodies from HIR annotations without normalizing their contents.
- [x] 1.2 Reuse those helpers alongside the existing derive/doc extraction path so passthrough attributes can be attached to both item-level and field-level render contexts.

## 2. Rust Code Generation

- [x] 2.1 Update the base Rust generator templates and renderers to emit collected passthrough attributes as `#[...]` immediately above generated items and fields.
- [x] 2.2 Review Rust-emitting backends (`rust`, `rust-axum`, `rust-jsonrpc`) and wire the shared passthrough emission into each backend where annotated declarations render Rust source.

## 3. Coverage and Documentation

- [x] 3.1 Add or update Rust snapshot fixtures to cover item-level passthrough, field-level passthrough, and stacked `@rust(...)` annotations using a serde-style example.
- [x] 3.2 Verify non-Rust generator outputs remain unchanged and document `@rust(...)` usage in the Rust generator docs with an example such as `@rust(serde(rename = "camelCase"))`.
