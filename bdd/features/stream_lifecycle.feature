@bdd_rest-errors
Feature: Structured streams retain terminal outcomes and declared business errors
  A ledger export can be locked before its first row or while rows are being read.
  An import succeeds only after a complete frame and validates the whole input.
  Error details use a nested wire name from an included contract file.

  Scenario Outline: Export and import preserve the contract across the first output boundary
    Given a REST IDL file "bdd/features/data/stream_lifecycle.idl"
    When I generate <language> code for the IDL
    Then the generated <language> code should be valid
    And I can run the generated <language> server using boilerplate
    And I can run hurl tests against the server
    And generated stream SDKs preserve terminal business outcomes for "<language>"

    Examples:
      | language |
      | rust     |
      | go       |
      | ts       |
