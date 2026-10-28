@bdd_rest-versioning-servers
Feature: API versioning — generated servers, the version header and version parameters

  Versioned servers run every API version behind one binary, round-trip the
  version header, and let each operation carry its own version parameter over
  real HTTP and WebSocket transports.

  Scenario: one binary serves every API version
    Given a versioned REST IDL file "bdd/features/data/versioned_rest.idl"
    When I generate rust code for the IDL
    Then I can run the generated rust servers for every API version using boilerplate
    Then I can run hurl tests for API version "v1" against the server
    And I can run hurl tests for API version "v2" against the server

  Scenario: the version header round-trips on every generated server
    Given a versioned REST IDL file "bdd/features/data/versioned_header.idl"
    When I generate rust code for the IDL
    Then the generated rust client pins the API version header
    And I can run the generated rust servers for every API version using boilerplate
    Then I can run hurl tests for API version "v1" against the server
    And I can run hurl tests for API version "v2" against the server

  Scenario Outline: generated SDKs pin each version parameter over real transports
    Given a version parameter contract carried in "<location>"
    When I generate both SDKs and OpenAPI for the version parameter contract
    Then every operation describes its own optional version parameter
    And the generated Rust SDK exchanges HTTP and WebSocket messages with both versions
    And the generated Go SDK exchanges HTTP messages with both versions
    And the generated TypeScript SDK exchanges HTTP and WebSocket messages with both versions

    Examples:
      | location |
      | header   |
      | query    |
      | cookie   |
