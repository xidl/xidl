@bdd_rest-serialization
Feature: HTTP representations and exceptions compose on one route
  Scenario Outline: Negotiation uses the returned representation and retains metadata
    Given a REST IDL file "bdd/features/data/http_envelope.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server using boilerplate
    Then I can run hurl tests against the server

    Examples:
      | lang |
      | rust |
      | ts   |
