## Context

`xidl-parser` currently preserves `PreprocInclude` in the typed AST, but `collect_defs` in `xidl-parser/src/hir/mod.rs` ignores it completely, so downstream HIR and codegen never see expanded include content. The intended behavior is for include to match C semantics: insert the target file content into the current tree in source order, and resolve include paths relative to the current file instead of the compiler process working directory.

This change spans HIR construction, file loading, and test fixture organization. It does not require modifying tree-sitter or typed AST include parsing. The main goal is to turn HIR construction from a single-file filtering pass into a recursively expanded file tree merge.

## Goals / Non-Goals

**Goals:**
- Expand `#include` during HIR construction while preserving the exact ordering position of definitions in the current file.
- Resolve relative paths from the directory of the including file and support nested includes.
- Require included files to parse into a valid typed AST; return explicit errors instead of silently skipping failures.
- Add parser/HIR/codegen coverage and extract duplicated test fixtures into shared include files.

**Non-Goals:**
- Do not implement the full C preprocessor macro system or attempt to evaluate conditional compilation branches.
- Do not inline include directives earlier in typed AST or change the existing syntax node structure.
- Do not define system header search paths, compilation databases, or complex include path configuration.

## Decisions

### 1. Introduce a file-context-aware expansion flow at the HIR layer

Keep `typed_ast::PreprocInclude` as it is and add a new HIR entry point that carries source file path context. That entry point reads included files, invokes the parser to produce typed AST, and then continues HIR collection. This reuses the existing typed AST parsing logic while keeping include semantics scoped to HIR construction, which matches the requirement.

The alternative was to expand include directly during parser or typed AST construction. That would push filesystem dependencies into a lower-level syntax parsing layer and blur the current plain-text-to-AST responsibility boundary, so it was not chosen.

### 2. Use depth-first, in-place definition merging

When `collect_defs` encounters `PreprocInclude`, it should immediately resolve the target file and merge its definitions into the current output vector at that exact point, effectively behaving like a textual in-place copy. Nested includes continue to expand depth-first under the same rule so the final HIR order matches source reading order.

The alternative was to collect all includes first and merge them at the end. That would violate the required insertion order and could change the visible ordering of pragmas, modules, and type declarations, so it was not chosen.

### 3. Resolve relative paths against the including file

`#include "foo.idl"` must be resolved relative to the directory of the current file. The expander therefore needs to carry the current source file path through recursion and use it to resolve the next include layer. This allows test fixtures to organize shared fragments by directory without depending on the runtime working directory.

The alternative was to resolve paths relative to the repository root or the CLI startup directory. That behavior is unstable and does not match the expected C-style include semantics, so it was not chosen.

### 4. Treat include failures as HIR construction errors

If the target file does not exist, cannot be read, fails typed AST parsing, or uses an unsupported include path form, HIR construction must fail with a diagnosable error. Silently ignoring the problem would produce incomplete HIR and shift later codegen failures away from the real root cause.

The alternative was to emit a warning and continue after include failure. Because include represents a required input dependency here, that approach would hide real errors, so it was not chosen.

### 5. Use shared IDL fixtures to validate downstream transparency of include expansion

The test plan needs to cover two kinds of behavior at the same time: direct parser/HIR assertions for include expansion and ordering, and `xidlc/tests` reuse of shared declarations through include files while proving codegen output remains identical to the version before shared fragments were extracted. That demonstrates include expansion is transparent to downstream generators.

## Risks / Trade-offs

- [Recursive or cyclic includes cause infinite expansion] -> Explicitly maintain an include stack in the design and implementation, and fail immediately when a repeated path is encountered.
- [Filesystem dependency makes the originally in-memory HIR conversion API more complex] -> Keep the existing pathless entry point for pure AST scenarios and add a path-aware entry point for CLI and test usage.
- [In-place ordering changes existing snapshots] -> Introduce changes only for cases that actually contain include directives and add ordering-focused snapshot and fixture tests.
- [Fixture refactoring affects many snapshots] -> Extract shared fragments conservatively and keep the final expanded HIR/codegen output unchanged to avoid unrelated snapshot drift.

## Migration Plan

Implementation should first add the new HIR construction entry point and failure paths, then add parser/HIR tests, and finally migrate duplicated fragments in `xidlc/tests` into shared include fixtures and refresh snapshots. If rollback is required, remove the new path-aware entry point and restore the test fixtures to inline definitions so the existing single-file parsing behavior returns.

## Open Questions

- Whether `#include <...>` and identifier-form include directives should initially be treated uniformly as unsupported file paths or whether they should reserve a separate search-path strategy.
- At which layer the current CLI should pass the root input file path into the new HIR construction entry point so parser unit tests and codegen tests share the same semantics.
