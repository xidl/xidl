---
name: code-style
description: Enforce repository-wide final validation before finishing a change or creating a commit. Use when work is complete, when preparing the final response, or immediately before committing. This skill requires `pre-commit run -a`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo tarpaulin --manifest-path xidl-parser/Cargo.toml --packages xidl-parser --include-files "xidl-parser/src/*" --exclude-files "xidl-parser/src/typed_ast/*" --fail-under 95 --out Stdout`, and `cargo publish --workspace --dry-run` to succeed from the repository root.
---

# Code Style

Use this skill as the final gate for repository changes.

## When To Use

Run this skill:

- after implementing code or documentation changes
- before telling the user the work is finished
- before creating a git commit

## Required Checks

From the repository root, run these commands and require all of them to pass:

```bash
pre-commit run -a
cargo clippy --all-targets --all-features -- -D warnings
cargo tarpaulin --manifest-path xidl-parser/Cargo.toml --packages xidl-parser --include-files "xidl-parser/src/*" --exclude-files "xidl-parser/src/typed_ast/*" --fail-under 95 --out Stdout
cargo publish --workspace --dry-run
```

## Workflow

1. Run `pre-commit run -a`.
2. If `pre-commit` rewrites files, stage those files.
3. If the rewrite happened after a commit was created, amend the current commit
   to include the rewritten files.
4. If `pre-commit` reports failures, fix the reported issues and rerun it until
   it succeeds.
5. Run `cargo clippy --all-targets --all-features -- -D warnings`.
6. If it fails, fix every warning or error and rerun it until it succeeds.
7. Run
   `cargo tarpaulin --manifest-path xidl-parser/Cargo.toml --packages xidl-parser --include-files "xidl-parser/src/*" --exclude-files "xidl-parser/src/typed_ast/*" --fail-under 95 --out Stdout`.
8. If it fails, fix the coverage gaps and rerun it until it succeeds.
9. Run `cargo publish --workspace --dry-run`.
10. If it fails, fix the packaging or manifest problem and rerun it until it
    succeeds.
11. Only finish the task or create a commit after all four commands succeed.

## Rules

- Do not skip any required check.
- Do not leave `pre-commit` rewrites unstaged or outside the current commit.
- Do not treat warnings as acceptable for the clippy step.
- Run the commands from the repository root unless the user explicitly asks for
  a narrower scope.
- If an environment problem blocks a required check, report the exact blocker
  and do not claim the change is complete.
