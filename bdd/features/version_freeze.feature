@bdd_rest-versioning
Feature: Version freeze release gate

  Every version's normalized OpenAPI document is checked in under
  `<contract>.versions/<value>.json`. Prose churn cannot read as a wire
  change: documentation keywords disappear at their keyword positions,
  unreferenced components are pruned and set-like arrays get a canonical
  order. `verify` fails CI when a released version drifts without the
  explicit `--update` door, and the door refuses to rewrite history.

  Scenario: freezing writes one copy per version value
    Given a versioned contract file "bdd/features/data/version_freeze.idl" in a scratch workspace
    When I freeze the contract
    Then the frozen copies exist for every version value

  Scenario: an unchanged contract passes the gate twice with the same bytes
    Given a versioned contract file "bdd/features/data/version_freeze.idl" in a scratch workspace
    When I freeze the contract
    And I freeze the contract again
    Then the frozen copies are byte-for-byte unchanged

  Scenario: verifying without frozen copies fails closed
    Given a versioned contract file "bdd/features/data/version_freeze.idl" in a scratch workspace
    When I verify the frozen contract and expect failure
    Then the failure tells how to create the frozen copies

  Scenario: prose edits do not read as drift
    Given a versioned contract file "bdd/features/data/version_freeze.idl" in a scratch workspace
    When I freeze the contract
    And I add a comment to the contract
    And I verify the frozen contract
    Then the release gate passes

  Scenario: a drifted released version fails the gate by name
    Given a versioned contract file "bdd/features/data/version_freeze.idl" in a scratch workspace
    When I freeze the contract
    And I rename the released operation
    And I verify the frozen contract and expect failure
    Then the drift report names the released version and the changed path

  Scenario: changing a released version's value fails the gate
    Given a versioned contract file "bdd/features/data/version_freeze.idl" in a scratch workspace
    When I freeze the contract
    And I change the released version value "2025-06" to "2025-07"
    And I verify the frozen contract and expect failure
    Then the drift report says the released version disappeared

  Scenario: appending a version needs the update door
    Given a versioned contract file "bdd/features/data/version_freeze.idl" in a scratch workspace
    When I freeze the contract
    And I append the version "2026-01"
    And I verify the frozen contract and expect failure
    Then the failure offers the update door
    When I freeze the contract
    Then the frozen copies exist for every version value

  Scenario: inserting before a released version is refused
    Given a versioned contract file "bdd/features/data/version_freeze.idl" in a scratch workspace
    When I freeze the contract
    And I insert the version "2024-12" before the released ones
    And I freeze the contract and expect failure
    Then the failure says released history is append-only

  Scenario: the diff reviews two versions structurally
    Given a versioned contract file "bdd/features/data/version_freeze.idl" in a scratch workspace
    When I run "version diff" for "v1" to "v2"
    Then the diff prints the structural difference
