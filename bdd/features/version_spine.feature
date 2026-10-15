@bdd_rest-versioning
Feature: Version spine release gate

  The version spine turns one declared history into the artifacts a release
  gate needs: a manifest to audit, a lock that freezes every released
  version, a diff that reviews one adjacent pair, and a verify that fails CI
  when a released version's wire identity drifts without the explicit
  `--update` door.

  Scenario: the spine manifests the declared history
    Given a versioned contract file "bdd/features/data/version_spine.idl" in a scratch workspace
    When I run "version manifest" on the contract
    Then the manifest pins the selector, the timeline and both surface digests
    And the manifest records how the versions correspond

  Scenario: an unchanged contract passes the freeze gate
    Given a versioned contract file "bdd/features/data/version_spine.idl" in a scratch workspace
    When I freeze the contract
    Then the lock file exists beside the contract
    When I verify the frozen contract
    Then the release gate passes

  Scenario: freezing an unchanged contract twice writes the same bytes
    Given a versioned contract file "bdd/features/data/version_spine.idl" in a scratch workspace
    When I freeze the contract
    And I freeze the contract again
    Then the lock file is byte-for-byte unchanged

  Scenario: verifying without a lock fails closed
    Given a versioned contract file "bdd/features/data/version_spine.idl" in a scratch workspace
    When I verify the frozen contract and expect failure
    Then the failure tells how to create the lock

  Scenario: a drifted released version fails the gate by name
    Given a versioned contract file "bdd/features/data/version_spine.idl" in a scratch workspace
    When I freeze the contract
    And I edit the contract so the v2 surface drifts
    And I verify the frozen contract and expect failure
    Then the drift report names the released version and both digests

  Scenario: a hand-edited lock digest fails the gate
    Given a versioned contract file "bdd/features/data/version_spine.idl" in a scratch workspace
    When I freeze the contract
    And I tamper with the frozen digest in the lock file
    And I verify the frozen contract and expect failure
    Then the drift report names the tampered digest and the real one

  Scenario: a renamed selector fails the gate
    Given a versioned contract file "bdd/features/data/version_spine.idl" in a scratch workspace
    When I freeze the contract
    And I edit the contract so the selector header renames
    And I verify the frozen contract and expect failure
    Then the drift report names the selector change

  Scenario: a changed version wire value fails the gate
    Given a versioned contract file "bdd/features/data/version_spine.idl" in a scratch workspace
    When I freeze the contract
    And I edit the contract so the v2 wire value changes
    And I verify the frozen contract and expect failure
    Then the drift report names the wire value change

  Scenario: the diff review classifies carries and flags coincidences
    Given a versioned contract file "bdd/features/data/version_spine.idl" in a scratch workspace
    When I run "version diff" on the contract
    Then the report shows the rename with its explicit-rename evidence
    And the report flags the wire coincidence for review

  Scenario: the diff view refuses a backwards review
    Given a versioned contract file "bdd/features/data/version_spine.idl" in a scratch workspace
    When I run "version diff --from 2025-06 --to 2025-01" on the contract and expect failure
    Then the failure says the timeline runs forward only
