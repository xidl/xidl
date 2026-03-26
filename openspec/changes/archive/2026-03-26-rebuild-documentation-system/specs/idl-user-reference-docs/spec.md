## ADDED Requirements

### Requirement: User documentation covers installation and generator workflows

The documentation set SHALL explain how to install `xidlc`, how to invoke it for
supported targets, and how to use `xidl-build` from Rust build scripts.

#### Scenario: User wants to install and run xidlc

- **WHEN** a reader opens the user documentation for setup
- **THEN** the docs provide supported installation paths and a minimal `xidlc`
  generation example

#### Scenario: Rust user wants build-time code generation

- **WHEN** a reader needs to integrate XIDL into a Rust project
- **THEN** the docs explain `xidl-build` usage, its main builder options, and
  the relationship between `build.rs`, output directories, and selected targets

### Requirement: User documentation teaches core IDL syntax and behavior

The documentation set SHALL explain the supported IDL syntax and behaviors in
plain language, including basic types, constructed types, annotations, optional
fields or parameters, and code generation expectations.

#### Scenario: User learns the language before generating code

- **WHEN** a reader studies the IDL user guide
- **THEN** the docs explain the major language elements with examples and
  describe how those elements affect generated outputs

#### Scenario: User looks up optional semantics

- **WHEN** a reader needs to understand `@optional`
- **THEN** the docs describe where it is supported, how it changes payload or
  generated-type behavior, and any relevant constraints

### Requirement: Reference documentation covers all supported language elements

The documentation set SHALL provide searchable reference coverage for supported
IDL elements, annotations, pragmas, transport annotations, and generator targets
that are implemented in the repository.

#### Scenario: Reader needs quick syntax lookup

- **WHEN** a reader searches for a specific IDL element or annotation
- **THEN** the docs contain a reference entry describing syntax, purpose, and
  any implementation-specific notes

#### Scenario: Reader needs target support lookup

- **WHEN** a reader wants to know which generators or targets are supported
- **THEN** the reference docs enumerate supported targets and the expected
  output or constraints for each target
