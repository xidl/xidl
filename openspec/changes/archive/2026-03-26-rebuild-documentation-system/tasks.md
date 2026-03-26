## 1. Documentation Architecture

- [x] 1.1 Redesign `mkdocs.yml` navigation to separate user docs, reference,
      RFCs, development docs, and AI skill content
- [x] 1.2 Add landing pages that explain audience, scope, and recommended
      reading order for each documentation section
- [x] 1.3 Audit existing docs and examples to map current coverage, missing
      topics, and obsolete placeholder pages

## 2. User Guides

- [x] 2.1 Write user documentation for `xidlc` installation, CLI usage, target
      selection, and end-to-end generation workflow
- [x] 2.2 Write user documentation for `xidl-build` in Rust, including
      `build.rs` usage and builder options
- [x] 2.3 Write or expand user documentation for core IDL syntax, basic types,
      composite types, annotations, and behavior including `@optional`
- [x] 2.4 Create a unified HTTP guide with subsections for HTTP, HTTP Stream,
      and HTTP Security
- [x] 2.5 Create a JSON-RPC guide with subsections for JSON-RPC, JSON-RPC
      Stream, and JSON-RPC security behavior

## 3. Reference Documentation

- [x] 3.1 Create searchable reference pages for supported IDL declarations,
      types, and annotations
- [x] 3.2 Add reference coverage for pragmas, transport annotations, and target
      generators
- [x] 3.3 Cross-link reference entries to relevant user guides, RFCs, examples,
      and implementation-backed notes

## 4. Contributor and AI Documentation

- [x] 4.1 Expand architecture documentation to describe the repository pipeline,
      crate responsibilities, and code generation stages
- [x] 4.2 Expand plugin development documentation with protocol details and an
      end-to-end example workflow
- [x] 4.3 Create an AI skill document for XIDL workflows and publish or link it
      from the documentation system

## 5. Validation

- [x] 5.1 Review all new docs against implementation, examples, and RFCs to
      remove unsupported claims
- [x] 5.2 Verify docs site navigation, internal links, and search
      discoverability
- [x] 5.3 Ensure the final docs set gives users enough information to generate
      supported target outputs and gives contributors enough information to
      develop plugins
