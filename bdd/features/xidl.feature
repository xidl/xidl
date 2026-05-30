Feature: XIDL Generation and Communication
  As a developer
  I want to generate code in multiple languages and ensure they can communicate
  So that I can build cross-language systems

  Scenario Outline: Generate and communicate via REST API
    Given a REST IDL file "bdd/features/data/complex_rest.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should contain correct User struct and UserService interface
    And I can run the generated <lang> server and client
    And the client can create a user with name "Alice" and id 1
    And the client can get the user with id 1 and see name "Alice"

    Examples:
      | lang   |
      | rust   |
      | go     |
      | python |

  Scenario Outline: Generate and communicate via JSON-RPC API
    Given a JSON-RPC IDL file "bdd/features/data/complex_jsonrpc.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should contain correct AddRequest struct and Calculator interface
    And I can run the generated <lang> server and client
    And the client can call calculate(1, 2, ADD) to get 3

    Examples:
      | lang |
      | rust |
