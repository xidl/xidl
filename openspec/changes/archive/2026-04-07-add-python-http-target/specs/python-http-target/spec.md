## ADDED Requirements

### Requirement: Python Target Split

The system MUST provide separate `python` and `python-http` generation targets.

#### Scenario: Plain Python generation excludes HTTP runtime concerns

- **WHEN** `xidlc` generates code for the `python` target
- **THEN** the output MUST contain language-level Python models and declarations
  without requiring the Python HTTP runtime

#### Scenario: Python HTTP generation includes transport-aware artifacts

- **WHEN** `xidlc` generates code for the `python-http` target
- **THEN** the output MUST include HTTP-aware request, response, client, server,
  and metadata artifacts needed for the repository-supported HTTP RFC subset

### Requirement: Python Runtime Location

The system MUST place shared generated-code runtime support under the top-level
`python/` directory.

#### Scenario: Generated Python code depends on in-repo runtime modules

- **WHEN** Python or Python HTTP output requires shared helpers
- **THEN** the generated code MUST import those helpers from runtime modules
  rooted under `python/`

### Requirement: Python HTTP Framework-Neutral Service Contract

The `python-http` target MUST expose a framework-neutral service contract for
generated HTTP interfaces.

#### Scenario: Generated service contract uses abstract methods

- **WHEN** `xidlc` generates a Python HTTP interface
- **THEN** it MUST emit an abstract service interface whose methods correspond
  to the effective HTTP operations of that interface

#### Scenario: User implementation subclasses generated service contract

- **WHEN** an application implements generated Python HTTP behavior
- **THEN** it MUST be able to subclass or concretely implement the generated
  abstract service interface without editing generated files

### Requirement: Python HTTP Automatic Route Registration

The `python-http` target MUST generate route-registration helpers that mount
service implementations automatically.

#### Scenario: Generated route registration binds implemented service methods

- **WHEN** a user passes a concrete generated service implementation to the
  generated registration API
- **THEN** the API MUST produce route bindings for the effective HTTP methods
  and normalized paths declared by the IDL

#### Scenario: Generated route registration centralizes transport logic

- **WHEN** generated Python HTTP routes are registered
- **THEN** request binding, response shaping, security extraction, and stream
  setup MUST be handled by generated code rather than requiring user-written
  per-route boilerplate

### Requirement: Python HTTP Framework Adapter Compatibility

The `python-http` target MUST stay framework-neutral while remaining directly
adaptable to Django and FastAPI.

#### Scenario: Generated registration integrates with Django through an adapter

- **WHEN** an application uses generated Python HTTP code with Django
- **THEN** the runtime and generated registration helpers MUST expose a narrow
  enough adapter surface for Django routing and request/response objects to
  integrate without regenerating target-specific code

#### Scenario: Generated registration integrates with FastAPI through an adapter

- **WHEN** an application uses generated Python HTTP code with FastAPI
- **THEN** the runtime and generated registration helpers MUST expose a narrow
  enough adapter surface for FastAPI routing and request/response objects to
  integrate without regenerating target-specific code
