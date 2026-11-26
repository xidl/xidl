@bdd_rest-versioning-core
Feature: version lock and check

  Scenario: adding later versions keeps locked version fingerprints stable
    Given a temporary workspace with a single-version IDL contract
    When I lock the contract versions with xidlc version lock
    And I append a second version with an added type to the contract
    Then xidlc version check succeeds with exit code 0

  Scenario: editing a locked version without lifecycle annotations fails check
    Given a temporary workspace with a single-version IDL contract
    When I lock the contract versions with xidlc version lock
    And I rename a field in the locked version without lifecycle annotations
    Then xidlc version check fails with exit code 1 reporting the drifted version

  Scenario: unforced relock refuses to overwrite drifted versions
    Given a temporary workspace with a single-version IDL contract
    When I lock the contract versions with xidlc version lock
    And I rename a field in the locked version without lifecycle annotations
    Then xidlc version lock without force fails with exit code 1
    And xidlc version lock with force updates the lock file
    And xidlc version check succeeds with exit code 0
