@bdd_rest-errors
Feature: Generated SDKs and OpenAPI describe the same wire contract
  Inventory changes retain positional stock, Unicode characters and typed union payloads.
  Invalid data is rejected before the same business logic runs in each server.

  Scenario Outline: One split contract works across every generated client and server
    Given a REST IDL file "bdd/features/data/wire_probe.idl"
    When I generate <language> code for the IDL
    Then the generated <language> code should be valid
    And I can run the generated <language> server using boilerplate
    And I can run hurl tests against the server
    And every Hurl response matches its generated OpenAPI contract
    And generated clients use the same "wire_probe" wire contract

    Examples:
      | language |
      | rust     |
      | go       |
      | ts       |
