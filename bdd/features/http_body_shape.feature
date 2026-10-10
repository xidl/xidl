Feature: Response body values and explicit upload media types
  Scenario Outline: Bare values preserve response headers and uploaded bytes
    Given a REST IDL file "bdd/features/data/http_body_shape.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server using boilerplate
    Then I can run hurl tests against the server

    Examples:
      | lang |
      | rust |
      | ts   |
