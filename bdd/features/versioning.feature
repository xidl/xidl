@bdd_rest-versioning-core
Feature: API versioning — code generation, version maps and contract validation

  A versioned IDL declares its history once and derives every API version from
  it. Code targets emit one module per version inside a single artifact, OpenAPI
  emits one document per version, and a version map renders the contract history.

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

  Scenario: versioned output can be narrowed to one API version
    Given a versioned REST IDL file "bdd/features/data/versioned_rest.idl"
    When I generate the "v1" API version of the IDL as openapi
    Then only the "v1" openapi document is generated

  Scenario: Go targets render every selected version module
    Given a versioned REST IDL file "bdd/features/data/versioned_rest.idl"
    When I generate go code for the IDL
    Then the generated go code should be valid

  Scenario: unimplemented versioning annotations are rejected
    Given a versioned REST IDL file "bdd/features/data/versioning_unsupported.idl"
    When I generate rust code for the IDL and expect failure containing "is not supported"

  Scenario: unsupported rename annotations fail without a version marker
    Given a versioned REST IDL file "bdd/features/data/versioning_renamed_from.idl"
    When I generate rust code for the IDL and expect failure containing "use @rename"

  Scenario: source validation rejects bare enum labels before emitting API versions
    Given a versioned REST IDL file "bdd/features/data/union_unqualified_case.idl"
    When I generate rust code for the IDL and expect failure containing "case label 'mms' must be qualified as Enum::member"
    And I generate typescript-rest code for the IDL and expect failure containing "case label 'mms' must be qualified as Enum::member"

  Scenario Outline: replacement links are rejected before code generation
    Given a versioned REST IDL file "bdd/features/data/versioning_replaces_<timeline>.idl"
    When I generate rust code for the IDL and expect failure containing "application hooks"

    Examples:
      | timeline  |
      | versioned |
      | plain     |

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
