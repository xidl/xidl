@bdd_rest-versioning
Feature: API versioning

  A versioned IDL declares its history once and derives every API version from
  it. Code targets emit one module per version inside a single artifact, OpenAPI
  emits one document per version, and one generated server serves every version.

  Scenario: code targets emit one module per API version
    Given a versioned REST IDL file "bdd/features/data/versioned_rest.idl"
    When I generate rust code for the IDL
    Then the generated rust code should contain the API version modules

  Scenario: openapi emits one document per API version
    Given a versioned REST IDL file "bdd/features/data/versioned_rest.idl"
    When I generate openapi documents for every API version
    Then each API version has its own openapi document

  Scenario: one binary serves every API version
    Given a versioned REST IDL file "bdd/features/data/versioned_rest.idl"
    When I generate rust code for the IDL
    Then I can run the generated rust servers for every API version using boilerplate
    Then I can run hurl tests for API version "v1" against the server
    And I can run hurl tests for API version "v2" against the server

  Scenario: versioned output can be narrowed to one API version
    Given a versioned REST IDL file "bdd/features/data/versioned_rest.idl"
    When I generate the "v1" API version of the IDL as openapi
    Then only the "v1" openapi document is generated

  Scenario: targets without versioned module support fail loudly
    Given a versioned REST IDL file "bdd/features/data/versioned_rest.idl"
    When I generate go code for the IDL and expect failure containing "cannot render one module per API version yet"

  Scenario: unimplemented versioning annotations are rejected
    Given a versioned REST IDL file "bdd/features/data/versioning_unsupported.idl"
    When I generate rust code for the IDL and expect failure containing "is not supported yet"

  Scenario: typescript version adapters serve old clients from one head implementation
    Given a REST IDL file "bdd/features/data/versioned_adapters.idl"
    When I generate ts code for the IDL with version adapters enabled
    Then the generated ts code should be valid
    And the generated ts adapters should require head decisions
    And the generated ts adapters should expose the version entry points

  Scenario: typescript version adapters stay off unless enabled
    Given a REST IDL file "bdd/features/data/versioned_adapters.idl"
    When I generate ts code for the IDL
    Then the generated ts code should not contain version adapters

  Scenario: version adapters refuse contracts the head can no longer serve
    Given a versioned REST IDL file "bdd/features/data/versioned_rest.idl"
    When I generate ts code for the IDL with version adapters enabled and expect failure containing "cannot serve removed operations"

  Scenario: one head implementation serves older versions through rust adapters
    Given a versioned REST IDL file "bdd/features/data/versioned_adapters.idl"
    When I generate rust code for the IDL with version adapters enabled
    Then I can run the generated rust servers for every API version using boilerplate
    And I can run hurl tests for API version "v1" against the server
    And I can run hurl tests for API version "v2" against the server
