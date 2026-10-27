@bdd_rest-versioning
Feature: Emitted Rust HTTP contracts compile without lint suppressions

  Scenario: ordinary required and optional HTTP parameters
    Given a REST IDL file "bdd/features/data/required_parameters.idl"
    When I generate rust code for the IDL
    Then the emitted Rust library passes strict clippy

  Scenario Outline: borrowed responses and versioned adapters
    Given a REST IDL file "bdd/features/data/<fixture>.idl"
    When I generate Rust with the independent adapter target
    Then the emitted Rust library passes strict clippy

    Examples:
      | fixture            |
      | borrowed_responses |
