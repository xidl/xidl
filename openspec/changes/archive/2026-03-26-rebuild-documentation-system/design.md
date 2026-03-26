## Context

XIDL already has substantial implementation surface area: core IDL code
generation, Rust build integration, HTTP/OpenAPI generation, Rust Axum runtime
support, JSON-RPC generation, plugin-based extensibility, and RFC drafts for
transport mappings. The published docs site, however, only exposes a small
subset of that surface through a README include, a few target notes, and a small
set of development pages. Important topics requested by users, such as IDL
syntax, `@optional`, transport security semantics, `xidl-build`, and plugin
authoring, are either scattered, incomplete, or absent.

This change is cross-cutting because it affects site navigation, document
taxonomy, multiple existing pages, new reference material, and an AI-facing
skill artifact. The design therefore needs to define how documentation is
structured, how it stays aligned with implementation, and how overlapping
content is split between user docs, reference docs, and RFCs.

## Goals / Non-Goals

**Goals:**

- Define a documentation architecture that makes it obvious where to find user,
  reference, RFC, development, and AI-oriented material.
- Give users a guided path from installation to writing IDL to generating code
  for supported targets.
- Cover core IDL syntax, annotations, behaviors, and transport-specific usage
  with simpler user docs, while preserving RFCs as the normative source for
  formal mapping rules.
- Provide a complete searchable reference that covers supported language
  elements, annotations, pragmas, generators, and transport concepts.
- Give contributors enough architecture and plugin documentation to understand
  the codebase and build new generators or plugins.
- Provide an AI skill document that reflects the repository’s terminology,
  workflows, and documentation map.

**Non-Goals:**

- Redefine transport semantics that already belong in existing RFCs.
- Introduce new generator behavior, annotation semantics, or plugin protocol
  changes as part of the documentation work itself.
- Guarantee a tutorial for every possible target beyond those implemented and
  evidenced in the repository.
- Replace examples and tests as the implementation source of truth.

## Decisions

- **Adopt a five-part documentation taxonomy.**
  - Decision: organize content into User Guides, Reference, RFCs, Development,
    and AI Skills.
  - Rationale: the current flat structure mixes audience types. Separating by
    audience matches the user request and reduces ambiguity about whether a page
    is explanatory, normative, or contributor-focused.
  - Alternatives considered:
    - Keep the current flat MkDocs nav and just add more pages. Rejected because
      it would increase sprawl without clarifying purpose.
    - Merge RFCs into user docs. Rejected because formal transport semantics
      need a stable normative home distinct from tutorials.

- **Use implementation-backed documentation rather than aspirational docs.**
  - Decision: derive user and reference coverage from existing generators,
    runtime code, examples, tests, and RFCs already in the repo.
  - Rationale: this reduces drift and keeps the doc set grounded in behavior the
    project actually supports today.
  - Alternatives considered:
    - Write forward-looking documentation for desired future features. Rejected
      because it weakens trust and makes the reference unreliable.

- **Split conceptual, procedural, and normative material explicitly.**
  - Decision: user docs explain concepts and workflows, reference docs catalog
    syntax and supported elements, and RFCs remain formal mapping documents.
  - Rationale: users need approachable explanations, while contributors and
    advanced users need precise lookup material. Mixing both in one page makes
    the docs harder to use.
  - Alternatives considered:
    - Put all detail into the reference and keep user docs minimal. Rejected
      because new users need narrative guidance, not just lookup tables.

- **Document transports as paired user guides plus RFC anchors.**
  - Decision: create user-facing transport guides for HTTP-family and
    JSON-RPC-family behavior, each with sub-sections for base mapping, stream
    semantics, and security, and cross-link them to RFC pages.
  - Rationale: transport support is central to XIDL’s value proposition, but the
    current docs are mostly RFC drafts or target notes.
  - Alternatives considered:
    - Keep only target-specific pages such as `rust-axum` and `rust-jsonrpc`.
      Rejected because transport concepts span generators, runtime, and schema
      documents.

- **Represent AI guidance as a first-class documentation artifact.**
  - Decision: add a skill document that summarizes repository context,
    vocabulary, supported generators, typical commands, source-of-truth files,
    and plugin-development guidance for AI agents.
  - Rationale: the user explicitly wants an AI-readable skill akin to shadcn’s
    skill pattern, and AI workflows work best when the repository provides a
    concise operational contract.
  - Alternatives considered:
    - Keep AI guidance implicit in general docs. Rejected because agents benefit
      from a compact, opinionated entry point with curated links and commands.

## Risks / Trade-offs

- [Risk] Documentation can drift from implementation as generators evolve ->
  Mitigation: anchor pages to examples, tests, RFCs, and concrete source files,
  and call out experimental features explicitly.
- [Risk] A large documentation set becomes hard to navigate -> Mitigation:
  enforce the information architecture in MkDocs nav and create landing pages
  that explain audience and reading order.
- [Risk] Reference coverage may duplicate existing RFC or target material ->
  Mitigation: define clear content roles and prefer cross-links over repeated
  normative prose.
- [Risk] AI skill content can become stale quickly -> Mitigation: keep the skill
  focused on workflows, source-of-truth locations, and stable concepts rather
  than copying long API descriptions.

## Migration Plan

- Rework MkDocs navigation and add landing pages for each audience segment.
- Audit existing docs, examples, RFCs, and implementation sources to build a
  coverage map for missing topics.
- Create the new user-guide, reference, contributor, and AI skill pages.
- Update existing pages or replace thin placeholders with fuller content and
  cross-links.
- Verify the generated site navigation and searchability before release.
- Rollback strategy: revert the navigation and new pages together if the new
  structure proves incomplete or misleading.

## Open Questions

- Should the AI skill be published only inside the repository, or also exposed
  on the public docs site as a discoverable page?
- Should the reference be a single large page, or several tightly scoped
  sections optimized for search and maintenance?
- How prominently should experimental transport behavior be labeled in user docs
  versus RFC pages?
