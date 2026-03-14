## ADDED Requirements

### Requirement: Security Snapshot Matrix Coverage
The `xidlc` test suite MUST include HTTP security fixtures that cover effective
security resolution across supported generator targets.

#### Scenario: Inheritance and override fixtures are present
- **WHEN** interface-level and operation-level HTTP security annotations affect
  the effective requirement set
- **THEN** the test suite MUST include fixtures covering inheritance,
  operation-level replacement, and explicit anonymous override via
  `@no-security`

#### Scenario: Multiple supported security schemes are represented
- **WHEN** `xidlc` supports multiple unary HTTP security annotations such as
  basic, bearer, api-key, or oauth2
- **THEN** the snapshot suite MUST include fixtures that verify those schemes
  are preserved through the relevant target outputs

### Requirement: Security Invalid-Combination Validation
The `xidlc` test suite MUST assert validation failures for invalid HTTP
security annotation combinations.

#### Scenario: Duplicate security annotations are tested explicitly
- **WHEN** the same security annotation is repeated where duplicates are
  forbidden
- **THEN** the test suite MUST include a dedicated invalid fixture and an
  assertion on the resulting validation error

#### Scenario: Conflicting anonymous and authenticated annotations are tested explicitly
- **WHEN** `@no-security` appears together with any authenticated security
  annotation on the same effective scope
- **THEN** the test suite MUST include a dedicated invalid fixture and an
  assertion on the resulting validation error

### Requirement: Security Coverage Includes Stream Operations
The `xidlc` test suite MUST verify that security annotations interact correctly
with HTTP stream operations.

#### Scenario: Stream operation inherits interface security
- **WHEN** a streamed HTTP operation is declared on an interface with inherited
  security annotations
- **THEN** the test suite MUST verify the generated output reflects the
  effective inherited security for that stream operation

#### Scenario: Stream operation clears inherited security explicitly
- **WHEN** a streamed HTTP operation uses `@no-security` to override inherited
  interface security
- **THEN** the test suite MUST verify the generated output reflects anonymous
  access for that stream operation
