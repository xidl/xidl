@bdd_rest-versioning
Feature: versioned booking chain
  A three-release booking contract exercising the walkthrough's chain shape:
  an uninvertible packaging change, a retired attendee roster served from
  storage, a mechanically linked organizer rename and an implied default.
  The application owns version negotiation, cross-release idempotency and
  the behaviour switch; xidl owns the conversions and their refusals.

  Scenario: the booking chain keeps storage facts and pure conversions
    Given a versioned REST IDL file "bdd/features/data/versioned_chain.idl"
    When I generate TypeScript with the independent adapter target
    Then the versioned booking chain keeps storage facts and pure conversions

  Scenario: the Rust booking chain keeps the same facts
    Given a versioned REST IDL file "bdd/features/data/versioned_chain_rust.idl"
    When I generate Rust with the independent adapter target
    Then the Rust versioned booking chain keeps the same facts
