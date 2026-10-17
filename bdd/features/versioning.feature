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

  Scenario: version-map emits deterministic adjacent contract facts
    Given a versioned REST IDL file "bdd/features/data/versioned_rest.idl"
    When I generate a version map for the IDL
    Then the version map should describe both versions and remain byte-stable

  Scenario: identical wire shapes preserve distinct source declarations
    Given a versioned REST IDL file "bdd/features/data/version_map_declarations.idl"
    When I generate a version map for the IDL
    Then the version map distinguishes replacements from surviving declarations
    And source trivia and version selection preserve the complete declaration map

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

  Scenario: the version header round-trips on every generated server
    Given a versioned REST IDL file "bdd/features/data/versioned_header.idl"
    When I generate rust code for the IDL
    Then the generated rust client pins the API version header
    And I can run the generated rust servers for every API version using boilerplate
    Then I can run hurl tests for API version "v1" against the server
    And I can run hurl tests for API version "v2" against the server

  Scenario: Go targets render every selected version module
    Given a versioned REST IDL file "bdd/features/data/versioned_rest.idl"
    When I generate go code for the IDL
    Then the generated go code should be valid

  Scenario: unimplemented versioning annotations are rejected
    Given a versioned REST IDL file "bdd/features/data/versioning_unsupported.idl"
    When I generate rust code for the IDL and expect failure containing "is not supported yet"

  Scenario: lint protects lifecycle spelling independently of generation
    Given a versioned contract with annotation mistakes
      """
      @version enum Versions { v1, v2 };
      struct Item { string id; @remvoed(Versions::v2) string legacy; };
      @optional interface Items { @gett(path="/items") Item read(); };
      """
    When I check the contract with xidlc lint
    Then lint reports the annotation mistakes without rewriting the contract
    And generation remains independent and correcting the typo changes the intended version

  Scenario: plugin diagnostics and IO failures obey explicit CI warning policy
    Given a versioned contract with annotation mistakes
      """
      @version enum Versions { v1, v2 };
      struct Item { @remvoed(Versions::v2) string legacy; };
      @optional interface Items { @gett(path="/items") Item read(); };
      """
    When I check the contract with xidlc lint
    Then lint reports the annotation mistakes without rewriting the contract
    And plugin warnings obey the CI policy across multiple files
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

  Scenario: an independent TypeScript adapter target preserves frozen HTTP contracts
    Given a versioned REST IDL file "bdd/features/data/versioned_adapters.idl"
    When I generate TypeScript with the independent adapter target
    Then one TypeScript Head implementation passes the unchanged adapter Hurl contracts

  Scenario: three-version business decisions preserve validation and committed outcomes
    Given a versioned REST IDL file "bdd/features/data/versioned_adapters_business.idl"
    When I generate TypeScript with the independent adapter target
    Then the three-version TypeScript application enforces wire and business boundaries

  Scenario: single-version adapters validate requests before an identity dispatch
    Given a versioned REST IDL file "bdd/features/data/versioned_adapters_single.idl"
    When I generate TypeScript with the independent adapter target
    Then the single-version TypeScript application validates before dispatch

  Scenario: an independent Rust adapter target preserves frozen HTTP contracts
    Given a versioned REST IDL file "bdd/features/data/versioned_adapters.idl"
    When I generate Rust with the independent adapter target
    Then one Rust Head implementation passes the unchanged adapter Hurl contracts

  Scenario: Rust adapters preserve committed non-Clone outcomes across three versions
    Given a versioned REST IDL file "bdd/features/data/versioned_adapters_rust_business.idl"
    When I generate Rust with the independent adapter target
    Then the three-version Rust application enforces wire and business boundaries

  Scenario: single-version Rust adapters preserve owned identity outcomes
    Given a versioned REST IDL file "bdd/features/data/versioned_adapters_single.idl"
    When I generate Rust with the independent adapter target
    Then the single-version Rust application validates before dispatch
