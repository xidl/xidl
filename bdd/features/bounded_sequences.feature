@bdd_rest-serialization
Feature: Billing batch limits hold at every wire boundary
  A shared module declares a maximum invoice batch size. Empty and partial
  batches remain legal; exceeding a bound must not reach the reservation
  service, even through aliases, nested containers or binary payloads.

  Scenario Outline: billing limits agree across generated SDKs and raw HTTP
    Given a REST IDL file "bdd/features/data/bounded_sequences.idl"
    When I generate <lang> code for the IDL
    Then the generated <lang> code should be valid
    And I can run the generated <lang> server using boilerplate
    Then I can run hurl tests against the server

    Examples:
      | lang |
      | rust |
      | ts   |
