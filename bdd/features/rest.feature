Feature: REST API Generation and Communication
  As a developer
  I want to generate REST API code with various features and ensure they work
  So that I can build robust microservices

  Scenario Outline: Basic REST Communication
    Given a REST IDL file "bdd/features/data/complex_rest.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server and client
    And the client can create a user with name "Alice" and id 1
    And the client can get the user with id 1 and see name "Alice"

    Examples:
      | lang   |
      | rust   |
      | go     |
      | python |

  Scenario Outline: REST with Unions and Attributes
    Given a REST IDL file "bdd/features/data/all_scenarios.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server and client
    And the client can create an item with name "Test" and payload message "Hello Union"

    Examples:
      | lang   |
      | rust   |
      | go     |
      | python |

  Scenario Outline: REST Attributes (Rust Only)
    Given a REST IDL file "bdd/features/data/all_scenarios.idl"
    When I generate rust code for the IDL
    Then the generated rust code should be valid
    And I can run the generated rust server and client
    And the client can set and get the "system_status" attribute to "ACTIVE"

    Examples:
      | lang |
      | rust |

  Scenario Outline: REST Streaming (Rust Only)
    Given a REST IDL file "bdd/features/data/streaming.idl"
    When I generate rust code for the IDL
    Then the generated rust code should be valid
    And I can run the generated rust server and client
    And the client can receive 5 ticks from the stream

    Examples:
      | lang |
      | rust |

  Scenario Outline: REST Form-urlencoded
    Given a REST IDL file "bdd/features/data/media_types.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server and client
    And the client can submit a form with name "Alice" and age 30

    Examples:
      | lang   |
      | rust   |
      | go     |
      | python |

  Scenario: TypeScript REST Schema Imports
    Given a REST IDL file "bdd/features/data/complex_rest.idl"
    When I generate ts code for the IDL
    Then the generated ts code should be valid
    And the generated ts iface zod file should import the model schemas
