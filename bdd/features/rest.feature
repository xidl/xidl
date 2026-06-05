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
      | lang |
      | rust |
      | go   |
      | ts   |

  Scenario Outline: REST with Unions and Attributes
    Given a REST IDL file "bdd/features/data/all_scenarios.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server and client
    And the client can create an item with name "Test" and payload message "Hello Union"

    Examples:
      | lang |
      | rust |
      | go   |
      | ts   |

  Scenario Outline: REST Attributes
    Given a REST IDL file "bdd/features/data/all_scenarios.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server and client
    And the client can set and get the "system_status" attribute to "ACTIVE"

    Examples:
      | lang |
      | rust |
      | go   |
      | ts   |

  Scenario Outline: REST Streaming
    Given a REST IDL file "bdd/features/data/streaming.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server and client
    And the client can receive 5 ticks from the stream

    Examples:
      | lang |
      | rust |
      | go   |
      | ts   |

  Scenario Outline: REST Form-urlencoded
    Given a REST IDL file "bdd/features/data/media_types.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server and client
    And the client can submit a form with name "Alice" and age 30

    Examples:
      | lang |
      | rust |
      | go   |
      | ts   |

  Scenario: TypeScript REST Schema Imports
    Given a REST IDL file "bdd/features/data/complex_rest.idl"
    When I generate ts code for the IDL
    Then the generated ts code should be valid
    And the generated ts iface zod file should import the model schemas

  Scenario Outline: REST Flatten Any and StructWithAny (Issue 171)
    Given a REST IDL file "bdd/features/data/issue_171.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server and client
    And the client can send flatten any payload with key "foo" and value "bar"
    And the client can send flatten struct with any field payload with key "foo" and value "bar"

    Examples:
      | lang |
      | rust |
      | go   |
      | ts   |

  Scenario Outline: REST E2E test via Hurl using boilerplate
    Given a REST IDL file "bdd/features/data/<idl>.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server using boilerplate
    Then I can run hurl tests against the server

    Examples:
      | idl              | lang |
      | complex_rest     | rust |
      | complex_rest     | go   |
      | complex_rest     | ts   |
      | city_rest        | rust |
      | city_rest        | go   |
      | city_rest        | ts   |
      | rest_server      | rust |
      | rest_server      | go   |
      | rest_server      | ts   |
      | rest_media_types | rust |
      | rest_media_types | go   |
      | rest_media_types | ts   |
      | e2e_test         | rust |
      | e2e_test         | go   |
      | e2e_test         | ts   |

  Scenario Outline: REST Bad Path - Not Found
    Given a REST IDL file "bdd/features/data/complex_rest.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server and client
    Then the client gets a 404 error with msg containing "not found" when requesting GET "/999"

    Examples:
      | lang |
      | rust |
      | go   |
      | ts   |

  Scenario Outline: REST Bad Path - Invalid Parameter
    Given a REST IDL file "bdd/features/data/complex_rest.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server and client
    # Rust says "Cannot parse...", Go says "invalid syntax" or similar. Both are 400.
    Then the client gets a 400 error with msg containing "invalid" when requesting GET "/abc"

    Examples:
      | lang |
      | go   |

  Scenario: REST Bad Path - Invalid Parameter (Rust)
    Given a REST IDL file "bdd/features/data/complex_rest.idl"
    When I generate rust code for the IDL
    Then the generated rust code should be valid
    And I can run the generated rust server and client
    Then the client gets a 400 error with msg containing "cannot parse" when requesting GET "/abc"

  Scenario: REST Bad Path - Invalid Parameter (TS)
    Given a REST IDL file "bdd/features/data/complex_rest.idl"
    When I generate ts code for the IDL
    Then the generated ts code should be valid
    And I can run the generated ts server and client
    Then the client gets a 400 error with msg containing "validation failed" when requesting GET "/abc"
