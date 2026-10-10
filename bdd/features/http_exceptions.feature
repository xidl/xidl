@bdd_rest-errors
Feature: Typed HTTP exception contracts
  Exceptions preserve their status, response metadata and JSON body on the wire.

  Scenario Outline: Conditional requests and multiple declared exceptions
    Given a REST IDL file "bdd/features/data/http_exceptions.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server using boilerplate
    Then I can run hurl tests against the server

    Examples:
      | lang |
      | rust |
      | ts   |
