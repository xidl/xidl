# AGENTS

## Scope

- This file MUST govern handwritten source files under `xidl-build/src/`,
  `xidl-jsonrpc/src/`, `xidl-parser/src/`, `xidl-parser-derive/src/`,
  `xidl-rust-axum/src/`, `xidl-typeobject/src/`, and `xidlc/src/`.
- This file MUST govern handwritten test files under `xidl-jsonrpc/tests/`,
  `xidl-parser/tests/`, `xidl-rust-axum/tests/`, `xidl-typeobject/tests/`,
  `xidlc/tests/`, and `python/tests/`.
- This file MUST govern generator templates under
  `xidlc/src/generate/**/templates/` and `xidl-parser/src/hir/templates/`.

## Code Style

- Every new handwritten Rust source file MUST stay at or below 300 lines.
- Every edited handwritten Rust source file already above 300 lines MUST NOT
  increase in line count.
- Generated files, snapshot files, and vendored files MUST NEVER be used as
  precedents for handwritten code.
- Each crate MUST contain one implementation per responsibility.
- Public items declared in `pub fn`, `pub struct`, `pub enum`, `pub trait`, and
  `pub type` MUST include Rustdoc.
- Rustdoc attached to a single public item MUST NOT exceed 20 lines.
- Runtime code in handwritten `src/**/*.rs` files MUST return typed errors
  instead of `unwrap`, `expect`, or panic-driven control flow.
- Error strings in handwritten `src/**/*.rs` files MUST name the failing
  contract, input, or subsystem.
- `#[allow(...)]` MUST appear on one item, one statement block, or one function.
- `#[allow(dead_code)]` is FORBIDDEN.
- Dead code is FORBIDDEN.
- Feature-gated public APIs MUST keep the same type signatures and semantic
  contracts across feature combinations.

## Tests And Coverage

- Production code and unit test bodies MUST live in separate files.
- A module with one test file MUST place that file at `mod/test.rs` or
  `mod/tests.rs`.
- A module with multiple test files MUST place them under `mod/tests/`.
- Snapshot assertions MUST stay in the crate that owns the rendered output.
- Every bug fix MUST add or update a regression test in the owning crate.
- The command `make test` MUST pass.
- The command `make test-coverage` MUST pass.
- The coverage threshold for the tarpaulin command above MUST stay at or above
  95%.
- The command `pre-commit run -a` MUST pass.
- The command `cargo clippy --all-targets --all-features -- -D warnings` MUST
  pass.
- The command `cargo publish --workspace --dry-run --allow-dirty` MUST pass.

## Module Architecture

- Each module file under `*/src/` MUST represent one responsibility.
- Parsing, semantic modeling, code generation, runtime transport, and CLI
  presentation MUST stay in separate modules.
- `typed_ast` MUST keep a one-to-one mapping with the concrete `tree-sitter-idl`
  AST shape.
- `typed_ast` MUST NOT invent synthetic nodes, collapse distinct parser nodes,
  or recover information by reparsing raw annotation text when that would depart
  from the actual `tree-sitter-idl` AST.
- Public `lib.rs` files MUST declare crate entry points and re-exports.
- Public `lib.rs` files MUST NOT contain feature logic, parsing logic, rendering
  logic, transport logic, or protocol flow logic.
- Functions and methods MUST NOT declare more than 4 parameters.
- Parameter sets with 5 or more inputs MUST be modeled as a domain struct,
  options struct, or builder.
- New behavior outside `utils.rs` and `utils/` MUST live on domain types, trait
  implementations, or builders.
- New top-level free functions outside `utils.rs` and `utils/` are FORBIDDEN for
  orchestration, mutation, validation, rendering, and protocol flow.
- Generated-output tests, parser tests, and runtime behavior tests MUST stay in
  the crate that owns the behavior.

## Generator Architecture

- Generator implementations under `xidlc/src/generate/` and
  `xidl-parser/src/hir/interface_codegen/` MUST use Rust for preprocessing,
  normalization, validation, and render-context assembly.
- Generator implementations under `xidlc/src/generate/` and
  `xidl-parser/src/hir/interface_codegen/` MUST use `minijinja` for template
  rendering.
- Templates under `xidlc/src/generate/**/templates/` and
  `xidl-parser/src/hir/templates/` MUST contain rendering statements and data
  access statements.
- Manually assembled Rust string fragments inside generator implementation code
  MUST stay below 20 characters per fragment.
