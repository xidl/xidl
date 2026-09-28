Feature: JSON-RPC API Generation and Communication
  As a developer
  I want to generate JSON-RPC API code with various features and ensure they work
  So that I can build cross-language systems

  Scenario Outline: Basic JSON-RPC Communication
    Given a JSON-RPC IDL file "bdd/features/data/complex_jsonrpc.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should contain correct AddRequest struct and Calculator interface
    And I can run the generated <lang> server and client
    And the client can call jsonrpc method "Calculator.get_history" and get an empty list
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

  Scenario Outline: JSON-RPC with Multiple Interfaces
    Given a JSON-RPC IDL file "bdd/features/data/multi_interface.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server and client
    And the client can call math.add(10, 20) to get 30
    And the client can save "hello" to store and get it back

    Examples:
      | lang |
      | rust |

  Scenario Outline: JSON-RPC Basic Methods
    Given a JSON-RPC IDL file "bdd/features/data/jsonrpc.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server and client
    And the client can call Calculator.add with a=4 and b=5 to get 9
    And the client can call Calculator.subtract with a=10 and b=3 to get 7

    Examples:
      | lang |
      | rust |

  Scenario Outline: JSON-RPC Complex Calculator
    Given a JSON-RPC IDL file "bdd/features/data/complex_jsonrpc.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server and client
    And the client can call calculate(1, 2, ADD) to get 3
    And the client can call calculate(10, 3, SUBTRACT) to get 7
    And the client can call jsonrpc method "Calculator.get_history" and get the values "3, 7"

    When the client sends the jsonrpc request
      """
      {"jsonrpc": "2.0", "method": "Calculator.calculate", "params": {"req": {"a": 1}, "op": "ADD"}, "id": 5}
      """
    Then the client receives a jsonrpc error with code -32602 and message containing "missing field"

    When the client sends the jsonrpc request
      """
      {"jsonrpc": "2.0", "method": "Calculator.calculate", "params": {"req": {"a": 1, "b": 2}, "op": "MULTIPLY"}, "id": 6}
      """
    Then the client receives a jsonrpc error with code -32602 and message containing "unknown variant"

    When the client sends the jsonrpc request
      """
      {"jsonrpc": "2.0", "method": "Calculator.calculate", "params": {"req": {"a": "x", "b": 2}, "op": "ADD"}, "id": 7}
      """
    Then the client receives a jsonrpc error with code -32602 and message containing "invalid type"

    And the client can call jsonrpc method "Calculator.get_history" and get the values "3, 7"

    Examples:
      | lang |
      | rust |

  Scenario Outline: JSON-RPC Error Handling
    Given a JSON-RPC IDL file "bdd/features/data/multi_interface.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server and client

    When the client sends the jsonrpc request
      """
      {"jsonrpc": "2.0", "method": "NoSuch.method", "params": {}, "id": 1}
      """
    Then the client receives a jsonrpc error with code -32601 and message containing "method not found"

    When the client sends the jsonrpc request
      """
      {"jsonrpc": "2.0", "method": "Math.add", "params": {"a": 1}, "id": 2}
      """
    Then the client receives a jsonrpc error with code -32602

    When the client sends the jsonrpc request
      """
      {"jsonrpc": "2.0", "params": {"a": 1}, "id": 3}
      """
    Then the client receives a jsonrpc error with code -32600

    When the client sends the jsonrpc request
      """
      {"jsonrpc": "2.0", "method": "Math.add", "params": {"a": 1, b"b": 2}, "id": 4}
      """
    Then the client receives a jsonrpc error with code -32700

    When the client sends the jsonrpc request
      """
      {"jsonrpc": "2.0", "method": "Math.add", "params": {"a": "x", "b": 2}, "id": 5}
      """
    Then the client receives a jsonrpc error with code -32602 and message containing "invalid type"

    When the client sends the jsonrpc request
      """
      {"jsonrpc": "2.0", "method": "math.add", "params": {"a": 1, "b": 2}, "id": 6}
      """
    Then the client receives a jsonrpc error with code -32601 and message containing "method not found"

    Examples:
      | lang |
      | rust |

  Scenario Outline: JSON-RPC Server Stream Typed Push Stream
    Given a JSON-RPC IDL file "bdd/features/data/server_stream.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And the generated <lang> code should contain a typed server stream for Feed.messages
    And the generated <lang> code should contain a typed bidi stream for Feed.echo
    And the generated <lang> code should contain a typed bidi stream for Feed.converse
    And I can run the generated <lang> server and client
    And the client can subscribe Feed.messages with count 3 and receive 3 stream items
    And the client can call Feed.ping to get "pong"
    And the client can bidirectional stream Feed.echo with 2 messages
    And the client can bidirectional stream Feed.converse with union events

    Examples:
      | lang |
      | rust |

  Scenario Outline: JSON-RPC Stream Annotation Conflicts
    Given a JSON-RPC IDL file "bdd/features/data/server_stream_conflict.idl"
    When I generate <lang> code for the IDL and expect failure containing "mutually exclusive"

    Examples:
      | lang |
      | rust |

  Scenario Outline: JSON-RPC Id-less Messages Are Silenced
    Given a JSON-RPC IDL file "bdd/features/data/server_stream.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server and client
    When the client sends the jsonrpc notification
      """
      {"jsonrpc": "2.0", "method": "Feed.ping", "params": {}}
      """
    Then the client receives no jsonrpc response within 0.5 seconds
    And the client can call Feed.ping to get "pong"
    And the client can subscribe Feed.messages with count 2 and receive 2 stream items

    Examples:
      | lang |
      | rust |
