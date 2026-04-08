# XIDL Architecture

This document describes the repository-level invariants of XIDL: the parts of
the system that are expected to stay stable for a long time, even while
features, RFCs, generator details, and APIs continue to evolve.

It is not a changelog and it is not a complete implementation guide. It is a
map of the architectural boundaries that contributors should preserve.

## Scope

XIDL is an IDL-first code generator built on OMG DDS IDL with XIDL-specific
extensions. One source contract can drive multiple outputs, including language
bindings, HTTP surfaces, JSON-RPC surfaces, and machine-readable specs.

The core repository invariant is:

> XIDL is organized as a top-down lowering pipeline from syntax to semantics to
> target rendering.

Today that pipeline is:

```text
tree-sitter-idl -> typed_ast -> hir -> codegen
```

For HTTP-oriented outputs, there is an additional protocol-semantic layer:

```text
tree-sitter-idl -> typed_ast -> hir -> http_hir -> http generators
```

This layered model is intentional. The project prefers explicit intermediate
representations over ad hoc target-specific interpretation because that keeps
the system easier to understand, easier to test, and easier to extend.

## Architectural Invariants

The following statements should remain true over time.

### 1. XIDL is a layered compiler/codegen pipeline

- Parsing is separate from semantic lowering.
- Semantic lowering is separate from rendering.
- Generators should consume stable intermediate representations, not raw parse
  trees.
- New capabilities should be introduced by adding or refining a layer, not by
  leaking protocol or target logic backward into earlier phases.

### 2. HIR is the shared semantic contract for generators

- `typed_ast` preserves structure close to the parsed IDL.
- `hir` is the normalized semantic form used for generation.
- Most generators should depend on `hir`, because it is a more stable and more
  intentional representation than syntax-shaped AST nodes.

### 3. Protocol semantics belong in shared protocol layers

- HTTP-specific understanding belongs in `http_hir`.
- HTTP generators such as Axum, Go HTTP, Python HTTP, and OpenAPI should render
  from shared HTTP semantics instead of each generator interpreting RFC rules
  independently.
- The generator's job is primarily rendering, not re-deciding protocol meaning.

This is why `http_hir` exists: it centralizes interpretation of the HTTP RFCs
and prevents language-specific divergence.

### 4. RFCs are the behavioral source of truth for XIDL extensions

- `docs/rfc/*.md` defines the semantics of XIDL extensions.
- RFCs specify behavior, mapping rules, and contract meaning.
- RFCs do not have to define the public API shape of every generated target.
- Generator-specific API shape remains a renderer concern, as long as renderer
  behavior stays compatible with the RFC-defined semantics.

### 5. Code generation is composable and unidirectional

- Data flows forward through the pipeline.
- Generators may produce final files, or produce another intermediate artifact
  for a later generator stage.
- A generator can handle only part of the work and hand the result to another
  generator through the IPC contract.
- The flow is still one-way: later stages do not mutate earlier layers.

### 6. Plugins are first-class generators

- `xidlc` uses a plugin architecture.
- Built-in generators and external generators follow the same driver model.
- Plugins communicate through the `Codegen` RPC contract defined in
  `xidlc/src/jsonrpc/ipc.idl`.
- External generators replace the rendering stage, not parsing or HIR creation.

### 7. Tests protect semantics, not just code paths

- Unit tests validate correctness of interfaces and transformations.
- Snapshot tests protect stable generated output against accidental changes.
- Snapshot drift is expected only when semantics intentionally change.

## Bird's-Eye View

At repository scale, XIDL is split into a few stable responsibility areas:

```text
IDL input
  -> xidl-parser
  -> shared semantic models
  -> xidlc driver
  -> built-in generators or external plugins
  -> generated code / schemas / specs
  -> runtime crates and examples
```

The workspace is not a collection of unrelated crates. It is one compiler and
codegen system split into layers and support crates.

## End-to-End Data Flow

The normal generation path is:

1. Parse IDL with `tree-sitter-idl`.
2. Build `typed_ast`.
3. Lower `typed_ast` into `hir`.
4. Start a generator session in `xidlc`.
5. Run a built-in generator or an external plugin through the same IPC-shaped
   contract.
6. Either emit files directly, or emit another intermediate artifact and
   continue the pipeline.
7. Write the final artifacts to disk.

The driver in `xidlc/src/driver/generate.rs` makes this explicit:

- generation starts from IDL text
- the initial semantic handoff is `hir`
- returned artifacts can be `Hir`, `HttpHir`, or `File`
- the driver recursively advances until only files remain

That recursion is an architectural invariant: XIDL supports staged generation,
not only one-shot rendering.

## HTTP Flow

HTTP-related code generation has an additional semantic projection:

1. Parse IDL.
2. Lower to `hir`.
3. Project `hir` into `http_hir`.
4. Hand `hir + http_hir` to HTTP-aware generators.
5. Render target-specific files.

`http_hir` exists so that:

- RFC interpretation happens once
- route, parameter, media type, stream, and security semantics are shared
- target generators stay aligned
- renderer complexity stays lower

Current consumers include the HTTP-oriented generators under
`xidlc/src/generate/`, especially Axum, Go HTTP, Python HTTP, and OpenAPI.

This pattern is expected to generalize. There is no `jsonrpc_hir` today, but it
would be architecturally consistent to add one in the future if JSON-RPC
semantics become complex enough to justify a shared protocol layer.

## Why `typed_ast`, `hir`, and `http_hir` Exist

These three layers exist because XIDL has to solve three different problems at
three different levels of stability.

- `typed_ast` answers: "Did we read the source correctly?"
- `hir` answers: "What does this IDL mean in a stable, generator-friendly form?"
- `http_hir` answers: "What does this contract mean after HTTP RFC semantics
  have been interpreted?"

They are not redundant models. Each layer deliberately removes one class of
concern and adds another.

### `typed_ast`

`typed_ast` is the syntax-near model that sits immediately above
`tree-sitter-idl`.

Its job is to preserve source structure while replacing parser-node mechanics
with structured Rust types.

What it removes:

- raw tree-sitter traversal details
- token-level parsing concerns
- concrete syntax tree noise that generators should never see

What it keeps:

- source-shaped declaration structure
- preprocessor constructs such as includes and pragma-like calls
- template-module constructs
- annotation structure close to the source
- doc-comment attachment near the parsed declaration

What it adds:

- a typed representation of the parsed source
- stable field access for later phases
- a clean seam for parser-focused tests and snapshots

Why it exists:

- parsing and semantic lowering should not be the same concern
- contributors need a way to verify that source text was parsed correctly before
  arguing about semantics
- syntax extensions should first affect the syntax layer, not immediately ripple
  through all generators

If `typed_ast` did not exist, parser code would be forced to mix syntax
handling, semantic lowering, and target concerns in one place.

### `hir`

`hir` is the shared semantic model for generation.

It takes the syntax-near `typed_ast` model and converts it into a normalized
representation that generators can depend on for a long time.

What it removes:

- preprocessor constructs in their raw source form
- include directives as directives
- syntax-only distinctions that do not matter to generation
- parser-shaped wrappers that are useful for reading source but not for
  reasoning about semantics
- template/preprocessor artifacts that are not part of the final semantic model

What it adds:

- normalized definitions used by generators
- typed `Pragma` values instead of raw preprocessor text
- resolved include expansion
- normalized annotations and names
- semantic helpers such as serialization/extensibility-related meaning
- optional interface expansion when a generator needs it

Why it exists:

- generators should depend on shared semantics, not on syntax accidents
- one semantic interpretation should feed many targets
- plugin IPC should carry a stable semantic contract rather than a
  syntax-shaped tree
- generator authors should not need to re-implement parsing, include handling,
  pragma interpretation, or normalization

If `hir` did not exist, every generator would need to understand parser-era
structures such as includes, raw pragma text, and syntax-specific variants.
That would duplicate logic, increase drift, and make the plugin contract much
less stable.

### `http_hir`

`http_hir` is a protocol-semantic projection from `hir`.

It exists because HTTP generation needs more than generic IDL semantics. It
needs an explicit, shared interpretation of the HTTP RFCs and XIDL's HTTP
extensions.

What it removes:

- generic IDL structure that is irrelevant to HTTP rendering
- raw operation and attribute forms before protocol interpretation
- repeated need for every HTTP generator to infer routes, bindings, and
  transport behavior on its own

What it adds:

- HTTP methods
- normalized routes
- request/response parameter partitioning
- path/query/header/cookie/body binding semantics
- stream semantics
- media-type semantics
- security semantics
- deprecation semantics
- document-level metadata such as package, version, and servers

Why it exists:

- HTTP semantics should be interpreted once and shared everywhere
- Axum, Go HTTP, Python HTTP, and OpenAPI should agree on the meaning of the
  same IDL contract
- renderers should focus on target-specific output, not on protocol inference
- HTTP bug fixes should land in one semantic layer instead of many renderers

If `http_hir` did not exist, every HTTP-oriented backend would need to
re-implement route inference, parameter-source inference, stream rules, and
security mapping. That would create inevitable behavioral drift across targets.

### Why the split is worth it

The three-layer split lets XIDL isolate three different kinds of change:

- syntax changes belong in `typed_ast`
- shared semantic changes belong in `hir`
- HTTP protocol-mapping changes belong in `http_hir`

That separation keeps the repository understandable and keeps regressions local.
It also makes tests more meaningful, because each layer can be validated for
its own responsibilities instead of only through final generated files.

## Code Map

This code map focuses on ownership boundaries rather than every file.

### Workspace Root

- `Cargo.toml`: workspace membership and shared dependencies.
- `README.md`: project overview and user-facing entry points.
- `ARCHITECTURE.md`: repository invariants and long-lived structure.
- `docs/`: user docs, developer docs, and RFCs.

### Parsing and Semantic Lowering

- `xidl-parser/`
  - owns parsing from `tree-sitter-idl`
  - owns `typed_ast`
  - owns `hir`
  - owns parser-focused tests and snapshots

Important subtrees:

- `xidl-parser/src/typed_ast/`: syntax-shaped typed AST
- `xidl-parser/src/hir/`: semantic HIR
- `xidl-parser/tests/`: parser and lowering tests

### Compiler Driver and Built-In Generators

- `xidlc/`
  - owns the CLI
  - owns diagnostics presentation
  - owns the generation driver
  - owns built-in generators
  - owns the plugin RPC contract

Important subtrees:

- `xidlc/src/driver/`: language resolution, generator orchestration, staging
- `xidlc/src/generate/`: built-in generators and intermediate generator stages
- `xidlc/src/generate/http_hir/`: shared HTTP semantic projection
- `xidlc/src/jsonrpc/ipc.idl`: generator IPC contract
- `xidlc/tests/`: codegen snapshot tests

### Runtime and Support Crates

- `xidl-build/`: `build.rs` integration for generation during Cargo builds
- `xidl-jsonrpc/`: runtime and transport support for JSON-RPC-based workflows
- `xidl-rust-axum/`: runtime support for generated Rust Axum code
- `xidl-xcdr/`: XCDR-related support
- `xidl-typeobject/`: type-object related assets and generation targets

These crates exist to support generated code and transport/runtime behavior.
They are not alternate compilers.

### Examples and Executable Contracts

- `xidlc-examples/`
  - sample IDL contracts
  - generated example artifacts
  - transport-focused tests
  - runnable example servers/clients

This crate acts as both documentation and an integration surface.

### Python and Other Language Runtime Helpers

- `python/`: Python runtime helpers and tests for generated Python-facing flows
- `golang/` if present in local workflows: Go-side runtime/codegen validation

## Layer Responsibilities

### `tree-sitter-idl`

Responsibility:

- parse OMG DDS IDL syntax into a concrete syntax tree foundation

Non-goal:

- own XIDL semantics or target-specific behavior

### `typed_ast`

Responsibility:

- provide a typed representation that still closely reflects parsed syntax

Use it when:

- syntax structure matters
- parser behavior is under test

### `hir`

Responsibility:

- represent normalized semantic meaning used by generation

Use it when:

- generator logic should reason about declarations and semantics instead of raw
  syntax shape

### `http_hir`

Responsibility:

- interpret HTTP RFC semantics once and expose a shared HTTP semantic model

Use it when:

- generating HTTP-oriented outputs
- mapping routes, request/response parameters, streams, media types, or
  security rules

### Final Generators

Responsibility:

- render target-specific files from shared semantic inputs

They may differ in API shape, idioms, and file layout, but they should not
disagree on the meaning of the contract.

## Rendering Model

`xidlc`'s built-in generators follow one rendering pattern:

- templates are written in Jinja and rendered with `minijinja`
- Rust code is responsible for walking `hir` or `http_hir`
- Rust code builds a render-friendly context before template execution
- templates are responsible for presentation, file layout, and simple branching
- semantic interpretation stays in Rust, not in templates

This is an important architectural boundary. The repository does not treat
templates as a second implementation language for core semantics.

### Why rendering is split this way

The split keeps generator behavior predictable:

- `hir` and `http_hir` stay the semantic source of truth
- normalization, protocol interpretation, and target-specific name resolution
  happen in Rust where they are typed and testable
- templates stay small enough to review as output shape rather than business
  logic
- the same semantic helper logic can be reused across many templates in one
  backend

In practice, a generator usually does three things:

1. Traverse `hir` or `http_hir`.
2. Build a target-specific context object or JSON value.
3. Call `renderer.render_template(...)` one or more times to emit files or
   fragments.

Examples of this pattern appear throughout `xidlc/src/generate/`:

- Rust, C, C++, Go, Python, TypeScript, Axum, Go HTTP, and Python HTTP each
  define a renderer around `minijinja`
- templates are embedded with `include_dir` and loaded by template name
- generator properties are exposed to templates through the `opt` global
- formatting helpers are added as Jinja filters/functions where needed

### Template responsibilities

Templates are expected to handle:

- target syntax layout
- repeated boilerplate
- straightforward conditional output
- iterating over already-prepared fields

Templates should not be the place that:

- re-derives semantic meaning from raw `hir`
- interprets RFC behavior independently per backend
- performs large amounts of target-specific graph traversal
- hides business rules that should be unit-tested in Rust

If a template needs complicated logic, that is usually a sign that the Rust
side should prepare a better context first.

## Plugin Architecture

`xidlc` treats generators as RPC services. This is true for both built-ins and
external plugins.

Stable properties of the plugin model:

- generator selection happens in the driver
- built-ins are started in-process over an inproc transport
- external plugins are started as child processes
- communication is done through the `Codegen` interface in
  `xidlc/src/jsonrpc/ipc.idl`
- artifacts are passed forward as `Hir`, `HttpHir`, or `File`

This means a plugin can:

- render files directly
- transform one stage into another stage
- participate in a longer pipeline instead of owning the whole pipeline

The important invariant is composability. Generators are pipeline stages, not
isolated monoliths.

### Session model

`xidlc` starts every generator through `CodegenSession`.

- built-in generators are hosted in-process and exposed over an inproc RPC
  transport
- external plugins are started as child processes such as `xidl-<lang>`
- both kinds of generators are wrapped as the same `Codegen` client from the
  driver's point of view
- version compatibility is checked through `get_engine_version()`

This gives the driver one orchestration model regardless of whether a stage is
compiled into `xidlc` or shipped separately.

### How plugins call each other

Plugins do not directly invoke each other. The driver is the orchestrator.

The flow is:

1. The driver starts a generator session for the current stage.
2. It calls `generate(hir, path, props)`.
3. The generator returns a sequence of `Artifact` values.
4. The driver inspects each artifact and decides the next step.

That next step depends on the artifact kind:

- `File`: generation for that branch is complete and the file is collected
- `Hir`: the driver starts another generator using the returned `lang`
- `HttpHir`: the driver stores `http_hir` into properties and starts the next
  generator using the returned `lang`

So the call graph is not plugin-to-plugin. It is:

```text
driver -> plugin A -> Artifact -> driver -> plugin B -> Artifact -> driver
```

This is why generator composition stays explicit and observable in one place.
Stages exchange typed artifacts, but stage transitions are owned by the driver.

### Multi-stage generation

Because generators can emit `Hir` or `HttpHir`, a backend can delegate part of
its work to another backend or to an intermediate semantic stage.

Examples:

- the initial `hir` stage parses and lowers the source before final generation
- HTTP-capable targets are routed through `http-hir` before Axum, Go HTTP,
  Python HTTP, or OpenAPI rendering
- the TypeScript generator can emit files and also return a `Hir` artifact for
  Rust generation of the non-interface portion

This staged model is the reason the driver recursively processes artifacts
until only `File` values remain.

### Property passing between stages

Plugin chaining is not only about the semantic payload. It also carries
properties forward.

- each stage contributes default properties through `get_properties()`
- the driver merges CLI/input metadata into those properties
- a stage may return extra `props` together with `Hir` or `HttpHir`
- later stages read those properties, often exposing them to templates as
  `opt`

This is how cross-stage metadata such as render options, timestamps, target
settings, and `http_hir` data flow through the pipeline without breaking the
layered architecture.

## Documentation Architecture

`docs/` contains three long-lived documentation classes:

- user documentation
- development documentation
- RFCs

The most stable contract among them is:

- user docs explain how to use the system
- development docs explain how to work on the system
- RFCs define extension semantics and expected behavior

RFCs are especially important because they are the factual standard for XIDL
extensions such as HTTP and JSON-RPC-related behavior.

## Testing Architecture

Testing is part of the architecture, not just a maintenance detail.

### Test Categories

#### 1. Parser and lowering unit/snapshot tests

Located mainly in `xidl-parser/tests/`.

Purpose:

- validate parser correctness
- validate `typed_ast` shape
- validate lowering into `hir`

Characteristic pattern:

- parse fixture text
- assert `typed_ast` snapshots
- assert `hir` snapshots

These tests protect the language front end and semantic lowering layers.

#### 2. Code generation snapshot tests

Located mainly in `xidlc/tests/`.

Purpose:

- protect generated outputs from accidental changes
- keep language generators stable
- make intentional output changes explicit in review

Layout invariant:

- language folders such as `c/`, `cpp/`, `rust/`, `ts/`, `golang-http/`,
  `python-http/`, `axum/`, `openapi/`, and `openrpc/` hold IDL inputs
- `shared/` contains reusable includes
- `snapshots/` stores approved output snapshots
- `codegen_snapshot.rs` discovers cases automatically

These tests are one of the most important guardrails in the repository.

#### 3. Integration and transport tests

Located mainly in `xidlc-examples/tests/`, plus runtime-crate test suites.

Purpose:

- validate generated artifacts in realistic flows
- verify protocol mappings such as security behavior and streaming behavior
- test executable server/client paths rather than only static output

Examples include:

- generated OpenAPI assertions
- HTTP snapshot tests driven by `.http` definitions
- JSON-RPC and HTTP example tests

### Why Snapshot Tests Matter

Snapshot tests are not cosmetic. Their main role is to prevent unintentional
output drift.

In a code generator, accidental output changes are architectural regressions
because downstream users treat generated code as part of the contract. Snapshot
tests make those changes visible immediately.

### Why Unit Tests Matter

Unit tests validate correctness of interfaces and transformations:

- parser behavior
- lowering behavior
- runtime behavior
- generator helper behavior
- protocol-specific semantic rules

The testing invariant is simple:

> XIDL tests both semantic correctness and output stability.

## Contributor Guidance

When adding or changing features, prefer changes that preserve these
architectural properties:

- keep the pipeline layered
- centralize shared semantics in intermediate representations
- keep generators focused on rendering
- preserve forward-only data flow
- extend the plugin protocol instead of bypassing it
- update snapshots only for intentional semantic or rendering changes
- add unit tests when changing transformation logic

## Non-Invariants

The following may change without violating the architecture:

- exact public API shapes of generated libraries
- exact file names emitted by a generator
- specific template structure inside a target renderer
- which targets are built-in versus external
- feature completeness of individual language backends

Those details can evolve. The invariants in this document are about structure,
ownership, and data flow.

## Summary

XIDL stays maintainable because it is built around explicit layers and shared
semantic models:

```text
tree-sitter-idl -> typed_ast -> hir -> protocol-specific IR -> rendering
```

The long-lived shape of the repository is:

- parse once
- normalize once
- interpret protocol semantics once
- render many targets
- protect semantics and output with unit tests and snapshots

That is the architectural center of gravity of this repository.
