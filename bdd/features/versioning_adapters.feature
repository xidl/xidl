@bdd_rest-versioning-adapters
Feature: API versioning — independent adapter targets preserve frozen HTTP contracts

  Independent TypeScript and Rust adapter targets implement a versioned Head
  service and enforce wire and business boundaries across three versions.

  Scenario: an independent TypeScript adapter target preserves frozen HTTP contracts
    Given a versioned REST IDL file "bdd/features/data/versioned_adapters.idl"
    When I generate TypeScript with the independent adapter target
    Then one TypeScript Head implementation passes the unchanged adapter Hurl contracts

  Scenario: strict business hooks preserve validation and committed outcomes across three versions
    Given a versioned REST IDL file "bdd/features/data/versioned_adapters_business.idl"
    When I generate TypeScript with the independent adapter target
    Then the three-version TypeScript application enforces wire and business boundaries

  Scenario: single-version adapters validate requests before an identity dispatch
    Given a versioned REST IDL file "bdd/features/data/versioned_adapters_single.idl"
    When I generate TypeScript with the independent adapter target
    Then the single-version TypeScript application validates before dispatch

  Scenario: an independent Rust adapter target preserves frozen HTTP contracts
    Given a versioned REST IDL file "bdd/features/data/versioned_adapters.idl"
    When I generate Rust with the independent adapter target
    Then one Rust Head implementation passes the unchanged adapter Hurl contracts

  Scenario: Rust adapters preserve committed non-Clone outcomes across three versions
    Given a versioned REST IDL file "bdd/features/data/versioned_adapters_rust_business.idl"
    When I generate Rust with the independent adapter target
    Then the three-version Rust application enforces wire and business boundaries

  Scenario: single-version Rust adapters preserve owned identity outcomes
    Given a versioned REST IDL file "bdd/features/data/versioned_adapters_single.idl"
    When I generate Rust with the independent adapter target
    Then the single-version Rust application validates before dispatch
