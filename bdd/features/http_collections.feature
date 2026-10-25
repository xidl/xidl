@bdd_rest-versioning
Feature: Collection wire occurrences agree with SDKs and OpenAPI

  Scenario Outline: collections retain empty elements and logical optionality
    Given a REST IDL file "bdd/features/data/http_collections.idl"
    When I generate <language> code for the IDL
    Then I can run the generated <language> server using boilerplate
    And I can run hurl tests against the server
    And collection SDKs preserve zero singleton and repeated inputs for "<language>"

    Examples:
      | language |
      | rust     |
      | ts       |
