@bdd_rest-serialization
Feature: Webhook publication composes with ordinary HTTP receivers
  The publication marker changes documentation only. The application owns
  event values, delivery identity, business validation and replay decisions.
  The same route also serves an unmarked read operation.

  Scenario Outline: a generated client and receiver handle delivery and replay
    Given a REST IDL file "bdd/features/data/webhook_http.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server using boilerplate
    Then I can run hurl tests against the server

    Examples:
      | lang |
      | rust |
      | ts   |
