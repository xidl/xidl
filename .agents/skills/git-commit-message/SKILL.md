---
name: git-commit-message
description: Draft or create git commit messages using conventional commits in a commitizen-compatible style. Use when the user asks for a commit message, wants help choosing a commit type or scope, or wants codex to commit changes with lowercase-first wording and BREAKING CHANGES footers instead of bang shorthand.
---

# Git Commit Message

Use this skill when the user wants to:

- draft a git commit message
- choose a conventional commit type or scope
- turn staged changes into a commit
- rewrite a commit message to match repo conventions

## Workflow

1. Inspect the changes first. Prefer `git diff --cached --stat` and
   `git diff --cached` when staged changes exist. If nothing is staged, inspect
   `git status --short` and the relevant diff before drafting a message.
2. Identify the dominant intent of the change. Choose one primary type such as
   `feat`, `fix`, `docs`, `refactor`, `test`, `build`, `ci`, or `chore`.
3. Choose a scope only when it adds useful precision. Good scopes are small and
   concrete, such as `xidlc`, `openspec`, `http`, or `skills`. Skip the scope
   when it adds noise.
4. Write the header as `type(scope): subject` or `type: subject`.
5. Add a body only when it adds useful detail that is not obvious from the
   header.
6. Add footers when required. Use `BREAKING CHANGES:` for incompatible behavior
   or API changes. Do not use `feat!:` or any other `!` shorthand in the header.

## Rules

- Follow Conventional Commits semantics and keep the output compatible with
  commitizen-style commits.
- Keep commit text lowercase by default.
- Preserve case only for proper nouns, acronyms, crate names, commands, API
  names, file paths, or other identifiers that would become incorrect if forced
  to lowercase.
- Keep the subject concise and specific. Prefer concrete subsystem names and
  behavior changes over vague summaries like `update code`.
- Do not mention implementation trivia in the subject when the user-visible or
  repository-level change is clearer.
- Do not add a body or footer unless it carries real information.
- If the change is breaking, explain the incompatibility in a
  `BREAKING CHANGES:` footer.

## Drafting Mode

If the user asks only for a commit message, return a draft message and do not
run git commands that create the commit.

Suggested output shapes:

Single-line:

```text
type(scope): subject
```

Multi-line:

```text
type(scope): subject

body paragraph

BREAKING CHANGES: migration or compatibility note
```

## Execution Mode

If the user explicitly asks to create the commit:

1. Inspect staged changes first.
2. Draft the message using the rules above.
3. If `commitizen` is available and the user wants that workflow, it is
   acceptable to use it.
4. Otherwise use a normal `git commit` command.

Prefer `commitizen` only as an execution option, not as a reason to change the
message format.

## Heuristics

- `feat`: new user-facing capability
- `fix`: bug fix or regression fix
- `docs`: documentation-only change
- `refactor`: code change without behavior change
- `test`: tests added or updated
- `build`: build, packaging, or dependency tooling changes
- `ci`: ci workflow changes
- `chore`: repository maintenance that does not fit the above cleanly

When a change spans multiple areas, pick the type that best describes the main
reason the commit should exist.

## Examples

```text
feat(xidlc): add openapi 3.2 stream schema support
```

```text
test(http): expand security and stream coverage
```

```text
refactor(skills): add conventional commit drafting workflow
```

```text
feat(api): rename client-stream annotations

BREAKING CHANGES: replace legacy underscore annotation names with hyphenated forms
```
