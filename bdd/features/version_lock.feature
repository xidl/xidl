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

  Scenario: locking selected versions leaves later versions editable
    Given a temporary workspace with a single-version IDL contract
    When I append a second version with an added type to the contract
    And I lock only version "2026-12-01"
    Then the lock contains only "2026-12-01"
    When I rename a field added in the second version
    Then checking only version "2026-12-01" succeeds
    And checking only version "2027-06-01" reports it is not locked
    When I lock only version "2026-12-01"
    Then the lock contains only "2026-12-01"

  Scenario: an explicit empty version set clears the lock
    Given a temporary workspace with a single-version IDL contract
    When I lock the contract versions with xidlc version lock
    And I lock the explicit empty version set
    Then the lock contains no versions and the empty version check succeeds

  Scenario: OpenAPI fingerprints ignore documentation and reject changes to the wire
    Given a temporary workspace with a single-version IDL contract
    When I lock the generated OpenAPI contract
    And I document and deprecate a field in the contract
    Then the OpenAPI fingerprint still matches
    When I rename a field in the locked version without lifecycle annotations
    Then checking the OpenAPI fingerprint reports drift

  Scenario: an older lock requires an explicit re-lock
    Given a temporary workspace with a single-version IDL contract
    When I lock the generated OpenAPI contract
    And I remove the algorithm version from the lock
    Then xidlc diagnoses the old lock before comparing fingerprints
    When I explicitly regenerate the lock
    Then xidlc version check succeeds with exit code 0
