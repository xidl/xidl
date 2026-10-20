@bdd_rest-serialization
Feature: Struct inheritance keeps one flattened JSON shape
  Scenario Outline: Inherited fields survive HTTP models and validators
    Given a REST IDL file "bdd/features/data/http_struct_inheritance.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server using boilerplate
    Then I can run hurl tests against the server

    Examples:
      | lang |
      | rust |
      | ts   |
      | go   |
