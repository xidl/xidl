@bdd_rest-versioning-core
Feature: version diff

  Scenario: version diff exposes contract changes with stable headers
    Given a version diff IDL file "bdd/features/data/version_diff.idl"
    When I compare version "v1" to "v2" with xidlc version diff
    Then the version diff exits with 1 and contains the contract changes

  Scenario: identical projections produce no output
    Given a version diff IDL file "bdd/features/data/version_diff.idl"
    When I compare version "v2" to "v3" with xidlc version diff
    Then the version diff exits with 0 and no output

  Scenario: source formatting and comments do not affect the diff
    Given a version diff IDL file "bdd/features/data/version_diff.idl"
    When I compare the original and reformatted version diff sources
    Then both version diffs are byte identical

  Scenario: unknown versions fail with declared choices
    Given a version diff IDL file "bdd/features/data/version_diff.idl"
    When I compare version "missing" to "v2" with xidlc version diff
    Then the version diff exits with 2 and lists the declared versions

  Scenario: version values select the same comparison as enum members
    Given a version diff IDL file "bdd/features/data/version_diff.idl"
    When I compare version "2026-04" to "2026-07" with xidlc version diff
    Then the version diff exits with 1 and contains the contract changes

  Scenario: broken contracts fail without partial output
    Given a version diff IDL file "bdd/features/data/version_diff.idl"
    When I compare invalid version diff contracts
    Then every invalid contract exits with 2 and no diff

  Scenario: output failures keep the documented error status
    Given a version diff IDL file "bdd/features/data/version_diff.idl"
    When I compare versions with a closed output pipe
    Then the output failure exits with 2 without a panic
