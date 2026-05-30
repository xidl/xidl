Feature: JSON-RPC API Generation and Communication
  As a developer
  I want to generate JSON-RPC API code with various features and ensure they work
  So that I can build cross-language systems

  Scenario Outline: Basic JSON-RPC Communication
    Given a JSON-RPC IDL file "bdd/features/data/complex_jsonrpc.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should contain correct AddRequest struct and Calculator interface
    And I can run the generated <lang> server and client
    And the client can call calculate(1, 2, ADD) to get 3

    Examples:
      | lang |
      | rust |

  Scenario Outline: JSON-RPC with Attributes
    Given a JSON-RPC IDL file "xidlc-examples/api/jsonrpc/city_jsonrpc.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server and client
    And the client can set and get the "firmware_channel" attribute to "stable"

    Examples:
      | lang |
      | rust |
