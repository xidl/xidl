@bdd_rest-media-and-any
Feature: OpenAPI unsigned long bounds
  Scenario: a published uint32 schema accepts its full wire range and rejects invalid values
    Given a REST IDL file "bdd/features/data/unsigned_long.idl"
    When I generate the unsigned integer OpenAPI contract
    Then its uint32 schemas accept the full unsigned range without changing other integers
