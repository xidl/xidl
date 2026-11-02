# xidl-http-docs

A separately installed XIDL generator that publishes one Starlight documentation
project from versioned HTTP contracts, including webhooks. It generates
Markdown, adjacent-version comparisons, raw reference files and AI indexes.
Starlight owns HTML rendering, navigation, search, syntax highlighting and code
copying.

## Generate and build

Building or installing the compiler and plugin requires Rust. Building the
resulting website additionally requires Node.js 24+ and pnpm 11+.

```sh
cargo install --path crates/xidl-http-docs
cargo install --path xidlc --features cli
XIDL_HTTP_DOCS_CONFIG=examples/documentation/xidl-http-docs.toml \
  xidlc gen -o generated/docs http-docs examples/documentation/xidl.toml
cd generated/docs
pnpm install --frozen-lockfile
pnpm build
pnpm preview
```

Deploy the contents of `generated/docs/dist/` at the configured deployment base.
Use `pnpm dev` for local writing and preview. Dependencies are pinned in the
emitted lockfile; installing them initially requires the package registry. The
Rust plugin does not run pnpm during Cargo builds and contains no compiled
browser bundle. The generated website serves local assets, without a CDN,
analytics or external fonts.

The example manifest selects two explicit API roots:

```toml
name = "billing-reference"
version = "0.1.0"
files = ["billing.idl", "payments.idl"]
```

The compiler resolves these files and imports into one compilation. Payments
imports Billing's v2 payment contract. Both APIs share a reference for that
type, one site overview, one search index and one AI index. Billing retains its
v1/v2 comparison. A manifest directory or several explicit IDL input files use
the same compilation-scoped generation path. Root API version selection uses
existing compiler options; a filter cannot select a version from an unversioned
root.

Compiler and plugin versions must match. The external generator uses the usual
NDJSON stdio protocol and opts into compilation-scoped generation. The compiler
supplies the import graph, typed HTTP projections and version-map facts in one
request.

## Document contracts

Write ordinary Markdown in the existing `///` documentation comments, before the
declaration and its annotations:

```idl
/// An invoice returned by **Billing**.
///
/// - Keep the `id` for later lookups.
/// - Read payment fields for the selected version.
struct Invoice {
    /// Stable invoice identifier.
    string id;
};
```

Paragraphs, lists and fenced code blocks remain Markdown in API references and
OpenAPI descriptions. The compiler owns comment collection; the HTTP renderer
consumes the same documentation metadata as other generators. Documentation text
does not create wire changes in version comparisons.

## Publication configuration

Place `xidl-http-docs.toml` in the working directory or provide its path through
`XIDL_HTTP_DOCS_CONFIG`. A manifest input does not implicitly select this file.
The library manifest owns source files and dependency mounts; this configuration
owns publication settings. Note paths resolve relative to the configuration.

```toml
title = "Billing API"
base_url = "https://example.com/reference/"
exclude = ["billing::Internal", "billing::Admin::erase"]
notes = ["changes/v1-to-v2.md"]
strict = false
full_max_bytes = 16777216
index_max_bytes = 262144
```

An absolute deployment path such as `/reference/` also works. An absolute HTTP
URL additionally enables canonical URLs and sitemap generation. Without a site
origin, Starlight reports that its sitemap integration is skipped.

Only types reachable from published operations are emitted. Exclusions apply to
references, change pages, navigation, search and AI output. A published
operation referencing an excluded type fails explicitly. Exclusion identities
use unit, module, interface and declaration names separated by `::`.

## Author change notes

Use ordinary Markdown with TOML front matter delimited by `+++`:

```markdown
+++
unit = "billing"
from = "v1"
to = "v2"
title = "Support multiple payments"
reviewed = true
impact = "Callers must handle a collection of payments."
upgrade = "Review callers that read payment_intent and use payments."
+++

An invoice can be associated with more than one payment.
```

Notes attach to a unit and adjacent version pair from the compiler's
version-map. Duplicate or unknown pairs fail. HTTP request fences must match a
published route, and JSON fences must parse, including fences with Starlight
title or line marker metadata. These checks cover Body, Impact and Upgrade.
Examples never send requests. Links and heading anchors are validated before
publication. Author HTML stays literal; heading HTML attributes and classes are
removed from the browser projection. Fenced code retains its original content.

Machine facts compare the published wire surface of adjacent projections. Shared
types have one fact with their known referencing operations. An added reachable
type means it first appears in the publication, not that its source was newly
written. Documentation and deprecation metadata are not wire changes.
Compatibility stays `Needs review`; author review is a separate state.
`strict = true` rejects changed pairs without a reviewed note containing Impact
and Upgrade. SDK methods, renames and migration mappings are never invented.

## Content and frontend ownership

The generator emits these artifacts from one publication:

- `src/content/docs/**/*.md`: Starlight content with explicit slugs and browser
  routes. Leading-underscore declarations and case-sensitive names keep their
  canonical identity. Version reference links preserve the logical entity.
- `public/openapi/{unit}/{version}.json`: self-contained, filtered OpenAPI for
  the corresponding `/playground/{unit}/{version}/` Scalar page. Shared imported
  models are included locally, and the selected wire version is prefilled.
- `public/**/*.md`, `public/llms.txt`, `public/llms-full.txt`: canonical raw
  Markdown and AI indexes. The full index contains all published pages; either
  byte limit fails generation rather than writing truncated output.

`frontend/` contains the packaged Starlight scaffold, lockfile and official
Tailwind v4 integration. The generated configuration consumes `xidl-site.json`.
A small Tailwind layer follows the XIDL blue palette and formats semantic tables
and native comparison details. Framework components provide the remaining UI.
Comparisons show Before/After summaries first, with canonical IDL declarations
in expandable panels and Expressive Code insertion/deletion markers. Copying
retains source bytes. Mobile comparisons stack, and ordinary reading and details
work without JavaScript. Pagefind search requires a production build; use
Ctrl/Cmd+K to open it. Pages provide links to raw Markdown, the AI index, the
selected version's Scalar playground and its OpenAPI download. Identical
declarations show No IDL changes; missing declarations have an explicit absent
state. IDL comes directly from the compiler's projected HIR and existing
printer.

Scalar uses pinned, locally bundled assets and the normal Starlight navigation.
Proxy, agent, telemetry, external fonts and persisted authentication are
disabled. Requests are sent only after reader interaction, and APIs must allow
the deployed site's origin through CORS. Ordinary Markdown reading works without
JavaScript; playgrounds require it.

The compiler records generated paths in its per-generator ownership manifest.
Regeneration overwrites owned files and removes stale owned files; it preserves
other files and rejects newly emitted paths that collide with user files. Do not
edit generated source files that appear in that manifest. Place custom assets at
other paths or change the scaffold in `frontend/` when maintaining the plugin.
Starlight owns `dist/`, so run `pnpm build` again after generation to refresh
HTML and search. Publication preflight rejects nested symlinks, traversal and
file conflicts. It does not provide a transaction across arbitrary filesystem
I/O failures or concurrent mutations.

## Validation

The owning crate keeps Markdown, navigation and comparison snapshots alongside
semantic, link and configuration regressions. The real CLI and browser BDD
journeys live in `bdd/features/http_documentation.feature`: multi-root
publication, repeat/narrow/restore, failed release recovery, copying, search,
version navigation, mobile, no-script reading, literal author content and real
Scalar requests to a local version-aware fake API, including missing records.
Run `just test-bdd` after installing the repository dependencies. CI installs
Chromium only for the shard containing documentation journeys.

The first transport adapter supports HTTP and webhooks through the existing
OpenAPI projection. Non-HTTP documentation receives an explicit unsupported
transport error. Parser semantics and runtime validation remain compiler-owned;
declared defaults and constraints do not promise server-side enforcement.
