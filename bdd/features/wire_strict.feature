@bdd_rest-serialization
Feature: JSON positions decode strictly while scalar transports coerce
  One contract, two transports: body members reject values whose JSON type
  differs from the declared type — exactly like the Rust target — while
  query parameters arrive as text and coerce into their declared types.

  Scenario Outline: the strict and coercing positions hold in every language
    Given a REST IDL file "bdd/features/data/wire_strict.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server using boilerplate
    Then I can run hurl tests against the server

    Examples:
      | lang |
      | rust |
      | ts   |
