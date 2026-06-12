Feature: CORS Support
  As a developer
  I want to ensure CORS headers are correctly applied based on IDL annotations
  So that I can control cross-origin access to my API

  Scenario Outline: CORS Wildcard
    Given a REST IDL file "bdd/features/data/cors.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server and client
    Then the response header "Access-Control-Allow-Origin" should be "http://example.com" when requesting OPTIONS "/any" with headers
      | name                          | value              |
      | Origin                        | http://example.com |
      | Access-Control-Request-Method | GET                |

    Examples:
      | lang |
      | go   |

  Scenario Outline: CORS Specific Origin
    Given a REST IDL file "bdd/features/data/cors.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server and client

    # Trusted origin
    Then the response header "Access-Control-Allow-Origin" should be "http://trust.me" when requesting OPTIONS "/trusted" with headers
      | name                          | value           |
      | Origin                        | http://trust.me |
      | Access-Control-Request-Method | GET             |

    # Untrusted origin
    Then the response header "Access-Control-Allow-Origin" should not be present when requesting OPTIONS "/trusted" with headers
      | name                          | value            |
      | Origin                        | http://evil.com |
      | Access-Control-Request-Method | GET              |

    Examples:
      | lang |
      | go   |
