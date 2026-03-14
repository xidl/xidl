### Requirement: Rust item annotations pass through to generated attributes
The Rust code generators MUST treat `@rust-xxx(...)` and `@rust-xxx` as
target-specific passthrough annotations and emit them as Rust outer attributes
`#[xxx(...)]` or `#[xxx]` on the generated item that corresponds to the
annotated IDL declaration.

#### Scenario: Struct annotation is emitted verbatim
- **WHEN** an IDL declaration such as a `struct`, `enum`, `union`, or interface
  carries `@rust-serde(rename_all = "camelCase")`
- **THEN** the generated Rust item MUST include
  `#[serde(rename_all = "camelCase")]` immediately above that item

#### Scenario: No-argument item annotation is emitted without parentheses
- **WHEN** an IDL declaration carries `@rust-inline`
- **THEN** the generated Rust item MUST include `#[inline]`

#### Scenario: Multiple item annotations preserve order
- **WHEN** an IDL declaration carries multiple `@rust-xxx` annotations
- **THEN** the generated Rust item MUST emit one `#[...]` line per annotation in
  the same order as the source annotations

### Requirement: Rust field annotations pass through to generated fields
The Rust code generators MUST emit `@rust-xxx(...)` annotations attached to IDL
members as Rust outer attributes on the generated field that corresponds to that
member.

#### Scenario: Member annotation becomes a field attribute
- **WHEN** an IDL member is annotated with
  `@rust-serde(rename = "camelCase")`
- **THEN** the generated Rust field MUST include
  `#[serde(rename = "camelCase")]` immediately above that field

### Requirement: Rust passthrough is isolated to Rust generation
`@rust-xxx(...)` and `@rust-xxx` annotations MUST affect generated Rust output
only and MUST NOT change generated artifacts for non-Rust targets.

#### Scenario: Non-Rust generators ignore rust passthrough
- **WHEN** the same IDL input is generated for non-Rust targets
- **THEN** those generated outputs MUST be unchanged apart from any existing
  generic annotation handling that does not depend on `@rust-xxx`
