## ADDED Requirements

### Requirement: Documentation site uses audience-oriented navigation

The documentation site SHALL expose first-level navigation for user
documentation, reference documentation, RFC documentation, development
documentation, and AI skill content so readers can identify the correct entry
point without reading repository source files.

#### Scenario: Reader opens the docs homepage

- **WHEN** a reader visits the documentation site
- **THEN** the site navigation presents clear sections for user docs, reference,
  RFCs, development docs, and AI skills

### Requirement: Documentation site provides guided entry points

The documentation site SHALL include landing or index pages that explain the
purpose of each documentation section and the recommended reading order for new
users and contributors.

#### Scenario: New user looks for where to start

- **WHEN** a reader wants to learn XIDL from scratch
- **THEN** the docs identify a user-oriented starting path from installation to
  code generation

#### Scenario: Contributor looks for architecture guidance

- **WHEN** a contributor wants to understand implementation and extension points
- **THEN** the docs identify a development-oriented starting path for
  architecture and plugin development

### Requirement: Documentation navigation distinguishes normative RFCs from user guides

The documentation system SHALL separate formal RFC documents from user-facing
guides and SHALL cross-link them when a user needs formal semantics.

#### Scenario: User needs precise transport semantics

- **WHEN** a reader starts from a user guide and needs formal transport rules
- **THEN** the guide links to the relevant RFC instead of duplicating the full
  normative definition
