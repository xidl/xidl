## Why

`xidlc` already preserves `#include` directives in the typed AST, but the HIR construction stage currently ignores them entirely. As a result, IDL cannot reuse shared declarations through relative paths the way C does. There are already multiple duplicated IDL fragments under `xidlc/tests`, which keeps increasing maintenance cost and means include semantics still have not been validated end to end.

## What Changes

- Implement `#include` expansion in `xidl-parser/src/hir`: read the included file, parse it into a valid typed AST, and merge its definitions into the current HIR.
- Resolve include paths relative to the directory of the file containing the directive, with behavior equivalent to inserting the target file content directly at the `#include` location.
- Preserve definition ordering semantics: included definitions must appear at the exact insertion point in the current file tree, rather than being appended or reordered during HIR aggregation.
- Add diagnostics for include failure cases, such as missing files, parse failures, or invalid generated ASTs, and stop expansion when they occur.
- Refactor duplicated IDL cases under `xidlc/tests`, extract shared fragments, reuse them through include, and add parser/HIR/codegen coverage for expanded include behavior.

## Capabilities

### New Capabilities
- `xidlc-include-expansion`: Expand `#include` directives during HIR construction using relative file paths while preserving in-place definition order.

### Modified Capabilities

## Impact

- Affected code: `xidl-parser/src/hir`, parser/HIR tests, and duplicated fixture files under `xidlc/tests`.
- Affected behavior: HIR generation becomes file-system aware when source files contain `#include`.
- Risks: recursive include handling, path resolution correctness, and preserving declaration ordering across nested includes.
