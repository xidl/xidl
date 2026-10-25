@bdd_rest-versioning
Feature: Inherited address contracts keep historical delivery facts
  A stable Head address and append-only delivery history serve every supported
  version. Only the decoding front end differs between a chain and version routers.

  Scenario Outline: Three-level inheritance through real HTTP and selected SDKs
    Given the inherited address application in "<mode>" mode
    When I start the inherited address application with independently selected SDKs
    Then the shared inherited address Hurl journey succeeds
    And inherited address responses match their generated OpenAPI contracts
    And the independent address SDKs preserve inherited history
    And inherited request validation preserves the documented mode boundary

    Examples:
      | mode  |
      | chain |
      | route |

  Scenario: TypeScript request adapters keep the same inherited write rules
    Given a REST IDL file "bdd/features/data/versioned_inheritance.idl"
    When I generate TypeScript with the independent adapter target
    Then the TypeScript inherited request journey preserves Head and stored facts
