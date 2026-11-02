# Versioned API documentation plugin

Status: draft. Scope: [#452](https://github.com/xidl/xidl/issues/452).

## Goal and ownership

Generate a complete, deterministic documentation site from the compiler's
projected contracts. Install the renderer separately as `xidl-http-docs`; invoke
it with `xidlc gen -o generated/docs http-docs path/to/xidl.toml`. The public
target is `http-docs` because its adapter documents HTTP APIs, including
webhooks.

The compiler owns parsing, import resolution, version selection, HTTP mapping,
wire names, and version-map facts. The plugin owns publication selection,
documentation links, change review, Markdown, a Starlight project scaffold and
AI indexes. A renderer must never reinterpret HTTP annotations or infer business
migrations.

## Primitives, from compilation to page

| Primitive           | Responsibility                                             | Composition and stability                                                     |
| ------------------- | ---------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Compilation input   | Supply all compilation units in one generation request     | Opt-in generator property; reusable by any compilation-scoped generator       |
| Contract projection | Supply each selected version and existing semantic input   | Reuses HIR and version-map; no second parser or lifecycle interpreter         |
| Published page      | One identity, version, title, Markdown body and references | Shared source for HTML, Markdown, navigation and AI output                    |
| Change fact         | Added, Removed or Changed with old/new contract evidence   | Separate from compatibility assessment and author prose                       |
| Source excerpt      | Select the real declaration for a published subject        | Reuses projected HIR and IdlPrinter; removes only known version wrappers      |
| Published OpenAPI   | Package one API unit/version for interactive exploration   | Same publication closure, self-contained references and selected wire version |
| Release note        | Author-supplied explanation and explicit review            | Joins a unit and adjacent version pair; never invents migrations              |
| Site assembly       | Resolve links and emit a deterministic artifact set        | One final assembly per compilation; rejects duplicate page identities         |

These boundaries are composable, general-purpose, orthogonal and stable. The
compilation input contains no site framework. A page contains no compiler
workflow. Change facts contain no business claims. Site adapters consume
published pages. No event bus is needed for a synchronous build: explicit input
and output artifacts provide the composition boundary without hidden lifecycle
state.

## Compiler integration

Add an opt-in `generation_scope = "compilation"` property and a versioned
`Compilation` semantic input to the existing NDJSON generator protocol. The
driver resolves the complete import graph first and reuses the existing HIR
generator for every unit. It sends the complete bundle to the external generator
once, then writes its file artifacts. Existing generators keep their unit-scoped
behavior.

Rust consumers with exhaustive matches on the public `CodegenInput` enum must
handle its new `Compilation` variant. The opt-in stdio behavior does not remove
this source-level API change; compiler and plugin versions must match.

The existing library loader expands the manifest's `files`, `exclude` and
dependency mounts before this compilation boundary. All selected API roots and
their imports enter one request and one website, sharing the overview,
references, navigation, search and AI indexes. A manifest directory or several
explicit IDL inputs use the same path. No per-file website or site-merging phase
is needed. The manifest owns the input set; `xidl-http-docs.toml` owns
publication settings. These remain separate responsibilities rather than adding
HTTP fields to the language's library manifest. The plugin reads
`XIDL_HTTP_DOCS_CONFIG`, or `xidl-http-docs.toml` in its working directory.
Manifest selection does not implicitly select publication settings.

The runnable [manifest example](../../examples/documentation/xidl.toml) contains
billing and payments API roots. Payments imports the billing v2 payment model;
the single site retains billing's v1/v2 comparison and shared model references.

Each compilation unit has a logical identity, explicit-root ownership, import
timeline metadata, version-map facts and typed per-version inputs. Machine paths
and declaration IDs are not page IDs. Unsupported compilation formats and
unsupported transport projections fail clearly.

## Documentation pipeline

1. Read the compilation input and publication configuration.
2. Consume existing HTTP projections and reuse the OpenAPI schema/operation
   adapter for effective wire shapes; retain HIR and version-map alongside it.
3. Build stable operation and type pages, with field anchors and shared links.
4. Join adjacent version facts with published operation reference ranges.
5. Attach author Markdown and report missing review/upgrade information.
6. Emit canonical raw Markdown, Starlight content with browser links and grouped
   AI indexes from the same page collection.
7. Build the emitted project with pnpm and deploy its dist directory.

The first transport adapter is HTTP. Non-HTTP contracts must receive an explicit
unsupported-transport error rather than HTTP-looking documentation.

Declaration documentation uses the compiler's existing `///` comments with
ordinary Markdown. The compiler collects these comments into shared semantic
documentation metadata; the HTTP adapter consumes that metadata for reference
pages and OpenAPI descriptions. Contract comparisons exclude documentation
metadata. Symbol-reference resolution and JSDoc authoring syntax remain outside
this implementation.

## Publication and determinism

Configuration supplies deployment URL, title, operation/type exclusions, release
notes and full-index limits. Only the selected public dependency closure enters
output. Excluded declarations referenced by published APIs are reported instead
of silently leaking private definitions.

Pages use logical unit, version and declaration names. Assembly uses sorted
collections, validates references and rejects path collisions. No timestamps,
absolute source paths or arbitrary traversal order enter published content.
`llms-full.txt` either contains every published Markdown page or generation
fails when its configured byte limit is exceeded.

## Browser presentation

Reuse the [XIDL website](https://xidl.netlify.app/) palette and system
typography: white background, blue accent, grouped sidebar, current-page state
and mobile navigation. Follow the
[Stripe changelog](https://docs.stripe.com/changelog) pattern of version
headings and a scan-friendly change table. Use native expandable before/after
panels and compiler-rendered IDL declarations. The overview starts with version
comparisons; tables show earlier and later contract summaries. Version menus
retain logical reference identity, and each reference links back to its related
changes. Markdown and HTML consume the same page facts.

The generator emits a Starlight project instead of maintaining a browser app.
The packaged `frontend/` scaffold contains Astro configuration, pinned pnpm
inputs, content schema and an official Tailwind integration. Starlight provides
navigation, Pagefind search, responsive layout, themes and Expressive Code.
There is no generated JavaScript committed to the Rust crate and no frontend
build during Cargo installation. Node.js 24+ and pnpm 11+ build the emitted
site.

Rust projects canonical Markdown links into browser routes through the Markdown
AST, preserving queries, fragments, external links and source code. Browser
links retain the current serving origin and configured deployment subpath, so
local preview navigation works with absolute production URLs. Canonical raw
Markdown and AI indexes retain the configured publication origin. Explicit slugs
preserve case and leading-underscore declarations. Heading IDs use the same
GitHub slug contract as Astro; smart punctuation is disabled to preserve source
text. Author HTML is literal, heading HTML attributes are removed, and content
stays Markdown rather than executable MDX. IDL declarations use native
expandable details and framework line markers; copying preserves the canonical
IDL source. The compiler-owned IdlPrinter renders projected HIR directly,
retaining business modules and interface ownership while removing only the
selected version wrapper. A missing declaration is explicit; identical source
says No IDL changes, including a newly reachable existing type. Raw Markdown and
AI files live under `public/` and are copied into `dist/`.

Each published API unit/version also receives a self-contained OpenAPI download
and an official Scalar Starlight playground. The renderer uses the same selected
operations, used security schemes and reachable model closure as reference
pages; imports become local component references. Exclusions also remove unused
tags. Schema traversal distinguishes user-defined property names from literal
examples, defaults and enums. The selected wire version is the playground's
parameter default; unversioned APIs do not acquire another API's version header.

The pinned Scalar browser bundle and MIT license are copied from the installed
package during pnpm build. The site disables proxy, agent, telemetry, external
fonts and persisted authentication. API requests occur only when a reader sends
a request; the API must permit the site's origin through CORS. Starlight route
middleware retains global API/version navigation on playground pages. A scoped
style lets Scalar dialogs escape the host content pane's isolation so their
controls remain accessible above the site header. The existing XIDL Shiki
grammar is shared by the website and generated IDL comparisons.

Long source lines use Expressive Code's built-in word wrapping; clipboard bytes
remain unchanged. Scalar secondary text and numeric highlighting use Starlight
theme tokens to maintain readable contrast across the embedded reference.
Starlight remains the theme authority. A wrapper preserves its built-in Head and
loads a small adapter that projects the resolved theme into Scalar's public body
mode classes. One MutationObserver reconnects after Astro swaps the body; the
adapter owns no preference storage or API-reference lifecycle. Scalar's
independent theme toggle is hidden.

The generated file manifest owns source publication; Starlight owns `dist/`.
After regenerating content, rebuild the site to remove stale HTML and search
entries. Existing unowned files are preserved, and conflicting new output paths
fail before writing. Nested symlinks and traversal are rejected. Filesystem I/O
is sequential, so arbitrary mid-write failures and concurrent changes are not
covered by a global atomic transaction.

## Publication ownership

Only explicit HTTP API roots publish release pages. Imported model-only units
supply reachable type pages without publishing their independent release lines.
Root API version filters do not select unrelated dependency versions. The driver
owns a per-generator path manifest, removes stale generated files after a
successful build, and preserves files outside that manifest.

## Change review

Machine facts, compatibility judgement and author prose are displayed
separately. Compatibility defaults to `Needs review` whenever transport
direction or meaning is not established. Deprecated declarations remain
referenceable. Documentation-only edits are not wire changes. Removed and added
fields are not paired as renames without evidence. Strict publication validation
rejects unreviewed changes and missing author impact or upgrade explanations.

## Validation

Business BDD uses the real CLI and plugin from an unrelated working directory.
Four long journeys cover equivalent multi-root inputs; build, browser, search
and source copying; narrow/repeat/restore including stale HTML and search;
invalid notes, examples, exclusions and limits without changing the last
publication; author HTML, heading attributes and fenced code. Chromium serves
only local production output. The fourth journey sends real HTTP requests to a
local fake service: v1/v2 success, a missing record, and an unversioned imported
model API. It verifies actual headers, response bodies, client-side navigation,
no automatic API calls and no external resource attempts. Host light/dark/Auto
changes preserve an open client response and override a conflicting Scalar
preference without rewriting it. Owner regression tests and snapshots cover
semantic envelopes, exclusions, Markdown routing, anchors, configuration, output
ownership and the distributed scaffold inventory. Two read-only reviewers
inspect compositions after implementation; the primary agent verifies findings
and owns all edits. Run the repository gates and report baseline exceptions
explicitly.

See the [implementation and acceptance report](versioned-api-docs-report.md) for
the primitive assessment, verified business scenarios and remaining limits.

## References

- [Issue #452](https://github.com/xidl/xidl/issues/452)
- [Plugin contract](../../xidlc/src/jsonrpc/ipc.idl)
- [Version-map model](../../xidl-parser/src/versioning/map/model.rs)
- [HTTP projection](../../xidl-parser/src/rest_hir/model/document.rs)
- [llms.txt proposal](https://llmstxt.org/)
- [CommonMark renderer](https://docs.rs/pulldown-cmark/)
- [Starlight](https://starlight.astro.build/)
- [Tailwind integration](https://starlight.astro.build/guides/css-and-tailwind/)
- [Expressive Code markers](https://expressive-code.com/key-features/text-markers/)
- [Expressive Code word wrap](https://expressive-code.com/key-features/word-wrap/)
- [Astro heading IDs](https://docs.astro.build/en/guides/markdown-content/#heading-ids)
- [OpenAPI nested-import bug #464](https://github.com/xidl/xidl/issues/464)
- [Documentation literal bug #465](https://github.com/xidl/xidl/issues/465)

- [Scalar Starlight integration](https://scalar.com/products/api-references/integrations/starlight)
- [Starlight route middleware](https://starlight.astro.build/guides/route-data/)
- [Scalar theme variables](https://scalar.com/products/api-references/themes)
- [Starlight component reuse](https://starlight.astro.build/guides/overriding-components/#reuse-a-built-in-component)
- [Astro after-swap](https://docs.astro.build/en/guides/view-transitions/#astroafter-swap)
