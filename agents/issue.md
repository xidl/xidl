# Issue Writing Guide

This document defines the categorization, title conventions, body structure,
terminology rules, and writing style for issues.

## 1. Classify the Issue

Identify the issue type before writing:

- **Bug**: The actual result does not match expectations (such as divergent
  behavior or conflicting output across code generators).
- **Feature**: The system lacks a required capability or functionality.

### Evaluation of RFCs

- **Maintainers evaluate RFC adoption**: Project maintainers decide whether a
  problem requires an RFC. The issue reporter does not make this determination.
- **Reporters may only suggest an RFC**: The reporter describes the discrepancy
  as a bug and may suggest an RFC as a potential follow-up in the
  recommendations section.

## 2. Title Conventions

Issue titles must include a standardized prefix:

- Bug: `[BUG] <brief summary>`
- Feature: `[FEATURE REQUEST] <brief summary>`

## 3. Bug Issue Structure

A bug issue body contains the following sections:

### 3.1 First Paragraph: Problem Statement

Describe the problem using this exact sentence pattern:

> Under [specific condition or input], expected behavior is [expected outcome].
> Actual result is [actual outcome].

The problem statement MUST adhere to these rules:

- State ONLY the observed incorrect phenomenon.
- Describing root causes, underlying explanations, or implementation mechanics
  is FORBIDDEN.
- Describing portions of the input or behavior that are valid, expected, or
  logical is FORBIDDEN.
- Describing the post-fix result, repaired outcome, or future state after
  resolution is FORBIDDEN.

### 3.2 Reproduction Steps and Concrete Examples

Provide concise reproduction steps immediately after the first paragraph.
Include concrete code snippets (such as IDL definitions and observed incorrect
outputs) to prevent abstract descriptions:

```text
Steps to reproduce:
1. Define a concrete IDL contract.
2. Run generator A and generator B.
3. Observe divergent outputs between targets.
```

### 3.3 Code Snippet Constraints

Code snippets in issues MUST adhere to these rules:

- Inlining or quoting code within prose sentences is FORBIDDEN.
- Quoted code snippets MUST precisely pinpoint the issue.
- A quoted code snippet MUST NOT exceed 20 lines.

**Pattern (correct):**

> Given the following IDL contract, the generated code fails to compile because
> line 12 in the output block contains unresolved symbol `Foo`.

**Pattern (forbidden):**

> Given the following IDL contract, `Foo` is logical, code `Bar` inside code
> `Baz` indirectly references `Qux` resulting in error E0072. After fixing, the
> generated code compiles cleanly.

### 3.4 Optional Closing: Recommendations

Suggest follow-up actions at the end when helpful:

- Add regression tests to lock in the expected behavior.
- Suggest an RFC to formalize cross-target contracts.

## 4. Feature Request Structure

A feature request issue body contains the following sections:

### 4.1 First Paragraph: Feature Description

Describe the requested feature using this exact sentence pattern:

> Support [feature name] to enable [specific use case or goal].

### 4.2 Concrete Examples

Provide concrete examples (such as IDL declarations and expected
representations) to demonstrate the requested capability clearly.

### 4.3 Optional Details: Design Limitations and Alternatives

Optionally note limitations in the current design or outline alternative
approaches:

- Current design limitations.
- Alternative implementation approaches.

## 5. Terminology and Error Rules

All issues MUST adhere to these terminology and error rules:

- Authors MUST ONLY use terminology and error descriptions that appear in
  repository code.
- Downstream compiler or runtime error codes (such as `E0072`) are FORBIDDEN.
- Placing downstream, third-party, or user error codes into issues is FORBIDDEN.

## 6. Style and Grammar Rules

All issues must adhere to these writing rules:

- **Keep content concise**: Keep descriptions, recommendations, and context
  brief.
- **Avoid verbosity**: Use direct, simple sentence structures.
- **Do not use parallel compound sentences**: Constructions such as "not X, but
  Y" are forbidden.
- **Use affirmative declarative sentences**: Express facts primarily through
  affirmative declarative statements.

## 7. Placement of Proposed Solutions

Proposals for bugs and feature requests follow these rules:

- **Post solutions in comments**: Keep the issue body focused on the problem or
  requirement. Post implementation proposals in issue comments.
- **Single-sentence overview**: Place a single sentence introducing the proposal
  directly before the solution heading.

### Example Proposal Comment

```markdown
This proposal adds a validation step to prevent route collisions.

### Route Validation Proposal

1. Inspect the existing route table before registration.
2. Return a validation error upon detecting duplicate paths and methods.
```
