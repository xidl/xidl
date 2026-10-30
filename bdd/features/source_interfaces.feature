@bdd_jsonrpc-and-rest-core
Feature: Source interfaces stay independent of transport declarations
  Interface declarations and imported models form the contract consumed by
  each transport projection.

  Scenario: Imported schemas keep their fields and union branches
    Given the source interface contract "xidlc/tests/openrpc/source_interfaces.idl"
    When I generate its OpenRPC documents twice
    Then imported model names preserve every declared field and union branch
    And empty calls, parameter directions and attributes preserve their contracts

  Scenario: Published versions keep their own operation sets
    Given the source interface contract "xidlc/tests/openrpc/versioned_calc.idl"
    When I generate its OpenRPC documents twice
    Then the versioned OpenRPC methods match their declared lifetimes
