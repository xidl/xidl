@bdd_rest-versioning-http
Feature: Required HTTP inputs never acquire fabricated scalar defaults

  Scenario Outline: generated servers reject absent inputs before dispatch
    Given a REST IDL file "bdd/features/data/required_parameters.idl"
    When I generate <language> code for the IDL
    Then I can run the generated <language> server using boilerplate
    And I can run hurl tests against the server
    And the required parameter SDK preserves explicit zero and absent values for "<language>"

    Examples:
      | language |
      | rust     |
      | ts       |
