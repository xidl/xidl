## ADDED Requirements

### Requirement: Development documentation explains architecture and extension points

The documentation set SHALL explain the repository architecture, compilation and
generation pipeline, key crates, and extension points needed to contribute new
features or generators.

#### Scenario: Contributor studies the compiler architecture

- **WHEN** a contributor reads the development docs
- **THEN** the docs explain the flow from IDL input through parser, HIR, and
  generator stages and identify the relevant crates or modules

### Requirement: Development documentation explains plugin development

The documentation set SHALL explain how to build generator plugins, including
protocol expectations, required RPC methods, invocation shape, and at least one
end-to-end example workflow.

#### Scenario: Contributor wants to implement a plugin

- **WHEN** a contributor reads the plugin development guide
- **THEN** the docs explain executable naming, transport expectations, required
  methods, data contracts, and a working implementation outline

### Requirement: Repository provides an AI skill document for XIDL workflows

The repository SHALL include an AI-oriented skill document that teaches an agent
how to navigate the docs, understand supported generators and transports, use
core commands, and find source-of-truth implementation files.

#### Scenario: Agent needs XIDL operating context

- **WHEN** an AI agent reads the skill document
- **THEN** it can identify the main docs sections, common generation commands,
  key examples, and the files that define plugin and transport behavior
