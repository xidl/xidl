# Documentation symbol links

Status: proposal implemented in a Draft PR; approval pending.

Requirement: [#478](https://github.com/xidl/xidl/issues/478), deferred from the
[symbol-link discussion in #462](https://github.com/xidl/xidl/pull/462#issuecomment-6594117427).

## Author and reader contract

An undefined Markdown reference in a `///` comment can name a published IDL type
or HTTP operation. The generated reference and AI indexes contain a navigable
link; OpenAPI descriptions retain the author's original Markdown.

```idl
/// Returns an [Invoice]. See [Invoices::list] for collection access.
@get(path = "/invoices/{id}")
Invoice retrieve(@path string id);
```

Supported references include `[Invoice]`, ``[`Invoice`]``, `[Invoice][]` and
`[the invoice][Invoice]`. Names resolve from the declaring scope toward the unit
root, and a leading `::` starts at that root. Local destinations remain in the
selected published API version. Imported references use the declared unit prefix
and its version binding; an explicit imported version selects that version.
Generation roots become visible to one another through imports.

Existing links and reference definitions take precedence. Code spans, fenced
examples and images retain their authored bytes. Unknown, excluded and
unpublished names remain ordinary Markdown. A declaration with no reference page
does not acquire a synthetic destination. A shadowing unpublished declaration
stops lookup, preserving its IDL identity.

## Ownership and composition

| Primitive                                 | Owner                             | Contract                                                               |
| ----------------------------------------- | --------------------------------- | ---------------------------------------------------------------------- |
| Semantic declarations and import bindings | Compiler HIR and CompilationInput | Supply identities, lexical scopes and selected versions.               |
| Published symbol destinations             | HTTP docs Symbols                 | Resolve names only to existing type and operation pages.               |
| Markdown reference expansion              | HTTP docs Markdown                | Expand undefined reference links using the scoped resolver.            |
| Field documentation provenance            | SchemaView / ReferenceRow         | Retain the declaring schema when fields are inherited or flattened.    |
| HTML, navigation and styling              | Starlight scaffold                | Render generated Markdown using the existing documentation components. |

The declaration catalogue is collected from the compiler's projected HIR through
DeclarationNames. Generated destinations reuse the existing Links and
DocumentationConfig contracts. Symbol resolution does not parse IDL source or
reconstruct API versions from serialized names.

The Markdown adapter uses pulldown-cmark's broken-link callback and source
offsets. It rewrites resolved reference spans, retaining surrounding Markdown
and examples. Each fragment carries its own source scope: interface prose uses
the interface, operation and parameter prose use the operation, and inherited
fields use the declaring type. Wire schema examples stay unmodified.

## Primitive assessment

- **Composable:** the destination resolver and Markdown adapter compose with raw
  references, Starlight rendering and AI indexes through existing URL contracts.
- **General-purpose:** names and destinations use IDL identities, independent of
  the API's application domain.
- **Orthogonal:** compiler semantics, publication visibility, Markdown parsing
  and website rendering retain separate owners.
- **Stable:** the public compilation envelope and frontend remain unchanged;
  syntax extends ordinary Markdown and has no additional configuration protocol.

The implementation adds no macros, dependencies or test framework. Existing
owning-crate snapshots and business journeys remain useful at their respective
rendering and publication boundaries. Acceptance uses the actual compiler and
plugin followed by a production Starlight build and browser navigation.

## Business acceptance

The actual compiler and plugin passed 29 publication checks covering both API
versions, bound and explicit imports, unrelated roots, inherited and flattened
field documentation, unpublished and excluded declarations, Markdown precedence,
code preservation, OpenAPI source text, idempotent generation and stale-page
removal. A production Starlight build and 11 Chromium checks confirmed rendered
links, versioned fields, navigation, clipboard contents and mobile layout with
no JavaScript errors or failed local resources.

These journeys exposed and corrected nested-link precedence, nested-image
handling, excluded-operation shadowing and collapsed-reference source spans. Two
read-only reviewers checked the resulting composition; the main agent verified
the findings against source and actual generated output. Existing tests retain
distinct rendering, publication and navigation coverage. This follow-up adds no
test source files.

## Sources

- [Rustdoc item links](https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html)
- [pulldown-cmark broken-link callback](https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Parser.html#method.new_with_broken_link_callback)
- `xidlc/src/jsonrpc/compilation.rs`
- `xidl-parser/src/hir/declarations/mod.rs`
- `crates/xidl-http-docs/src/symbols.rs`
- `crates/xidl-http-docs/src/markdown/symbols.rs`
