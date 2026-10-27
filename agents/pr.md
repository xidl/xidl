# PR Writing Guide

This document defines the requirements, target branch rules, structure,
terminology rules, and character limits for pull requests.

## 1. Classify the PR

Identify the PR type before writing:

- **bugfix**: The change corrects behavior that did not match expectations.
- **feature**: The change adds a new capability.
- **refactor**: The change restructures existing code without altering behavior.
- **docs**: The change updates documentation only.
- **chore**: The change updates tooling, dependencies, or configuration.

## 2. Target Branch and Lifecycle

### 2.1 Target Branch

All pull requests MUST target `master`. Target branches other than `master` are
FORBIDDEN.

### 2.2 Bugfix Prerequisites

- A bugfix PR MUST link to an existing issue created according to
  `agents/issue.md`.
- The first line of the PR description MUST be `Fixes #<issue_number>`.
- Opening a bugfix PR without an existing issue filed under `agents/issue.md` is
  FORBIDDEN.
- The implementation MUST strictly match the agreed outcome in the issue.

### 2.3 Feature Prerequisites and Draft PRs

- A feature PR MUST link to a design proposal in the project wiki.
- The first line of the PR description MUST be `Design: <wiki_url>`.
- While a feature design proposal is in the design phase (under discussion or
  exploratory prototype), the pull request MUST be created as a Draft Pull
  Request (`Draft PR`).
- Submitting a ready-for-review (non-draft) PR for a feature whose design
  proposal is in the design phase is FORBIDDEN.
- Merging a feature PR before its design proposal is formally approved is
  FORBIDDEN.

## 3. Title Convention

PR titles MUST use conventional commit formatting with an optional scope:

- `<type>[(scope)]: <brief summary>`

Allowed types: `fix`, `feat`, `refactor`, `docs`, `chore`.

Examples:

- `fix(parser): reject unqualified union enum labels`
- `feat(axum): support custom middleware bindings`
- `docs: update PR creation guidelines`

## 4. Description Structure

### 4.1 Header

The first line of the PR description MUST state the prerequisite link:

- Bugfix: `Fixes #<issue_number>`
- Feature: `Design: <wiki_url>`
- Refactor / Docs / Chore: `Prerequisite: None` (or `Fixes #<issue_number>` if
  an issue exists)

### 4.2 Body for Bugfix

Bugfix PRs MUST include the following sections:

#### Problem Statement

State the problem in one or two sentences using this pattern:

> When [specific condition], [component] [incorrect behavior] instead of
> [correct behavior].

The problem statement MUST adhere to these rules:

- State ONLY the observed incorrect phenomenon.
- Describing root causes or underlying explanations is FORBIDDEN.
- Describing portions of input or behavior that are valid, expected, or logical
  is FORBIDDEN.

#### Solution

State the fix in one sentence:

> [Action verb] [mechanism or code location changed].

#### Reproduction

Numbered concrete reproduction steps (concrete inputs, commands, observed
failure).

#### Tests

State the BDD test scenario added or updated with filename.

### 4.3 Body for Feature

Feature PRs MUST include:

#### Goal Statement

State the goal in one sentence using this pattern:

> Support [capability] to enable [use case or requirement].

#### Solution

State the implementation approach in one sentence.

#### Example

A minimal IDL or API code snippet showing the new capability.

### 4.4 Body for Refactor, Docs, Chore

Refactor, docs, and chore PRs MUST state the change in a single-sentence
Summary:

> [Action verb] [what was refactored, documented, or updated] to [benefit or
> purpose].

### 4.5 Code Snippet Constraints

Code snippets in pull request descriptions MUST adhere to these rules:

- Inlining or quoting code within prose sentences is FORBIDDEN.
- Quoted code snippets MUST precisely pinpoint the issue.
- A quoted code snippet MUST NOT exceed 20 lines.

## 5. Terminology and Error Rules

All pull requests MUST adhere to these terminology and error rules across
titles, descriptions, and review comments:

- Authors MUST ONLY use terminology and error descriptions that appear in
  repository code.
- Downstream compiler or runtime error codes (such as `E0072`) are FORBIDDEN.
- Placing downstream, third-party, or user error codes into pull requests is
  FORBIDDEN.

## 6. Review Communication

When responding to review comments on a pull request:

- Responses MUST explain behavior in terms of the compiler pipeline (IDL input,
  semantic validation / pass, intermediate representation, or generated target
  output).
- Generic syntax tutorials of target languages lacking IDL context are
  FORBIDDEN.
- Technical explanations MUST directly answer the reviewer's specific question.
- When a reviewer requests an architectural change, the author MUST confirm the
  revised design approach in comments before pushing major rewrites.

## 7. Forbidden Content

The PR description MUST NOT contain:

- Downstream compiler or runtime error codes (such as `E0072`).
- Downstream, third-party, or user error codes.
- Inlined code within prose sentences.
- Quoted code snippets exceeding 20 lines.
- Descriptions of portions of input or behavior that are valid or logical.
- Reasoning, trade-off analysis, or design rationale.
- Business-domain terminology or product-specific context.
- Verification results such as "all tests pass" or benchmark numbers.
- Descriptions of intermediate implementation steps.
- Anything outside the designated structure.

## 8. Character Limit

The PR description MUST NOT exceed 800 characters.
