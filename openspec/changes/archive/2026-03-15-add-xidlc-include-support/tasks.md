## 1. HIR include expansion

- [x] 1.1 Add a HIR construction entry point that accepts the root source file path and carries file context through recursive expansion.
- [x] 1.2 Implement `PreprocInclude` handling in `xidl-parser/src/hir` so included typed AST definitions are merged at the include position in depth-first order.
- [x] 1.3 Resolve string-literal include paths relative to the including file and detect recursive include chains before expanding nested files.
- [x] 1.4 Return explicit HIR construction errors when an include target cannot be read, parsed into typed AST, or resolved as a supported file path.

## 2. Parser and HIR validation

- [x] 2.1 Add parser/HIR tests that verify include expansion preserves the exact ordering of local and included definitions.
- [x] 2.2 Add coverage for nested includes, missing files, invalid included ASTs, and cyclic include graphs.
- [x] 2.3 Update or add snapshots so include-bearing fixtures assert the final expanded HIR shape rather than silently dropping include directives.

## 3. xidlc fixture reuse

- [x] 3.1 Identify duplicated IDL fragments under `xidlc/tests` and extract shared declarations into reusable include files.
- [x] 3.2 Update representative `xidlc/tests` fixtures to consume the shared include files without changing the final generated outputs.
- [x] 3.3 Refresh codegen snapshots or targeted tests to confirm include-based fixtures still produce the expected downstream artifacts.
