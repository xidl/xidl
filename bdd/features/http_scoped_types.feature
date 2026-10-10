@bdd_rest-media-and-any
Feature: Rust HTTP types resolve relative to their IDL declaration
  Scenario: Generated nested modules preserve scoped request and response types
    Given a REST IDL file "bdd/features/data/http_scoped_types.idl"
    When I generate rust code for the IDL
    Then the generated rust code should be valid
    And I can run the generated rust server using boilerplate
    Then I can run hurl tests against the server
