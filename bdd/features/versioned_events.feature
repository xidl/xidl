@bdd_rest-versioning
Feature: versioned event splits
  One broad event kind splits into two precise ones. Old subscribers keep
  receiving the old name for both halves, each push keeps its own id, and an
  old caller's write states the one half that owns the name through the
  upcast decision.

  Scenario: the event split serves old subscribers under the old name
    Given a versioned REST IDL file "bdd/features/data/versioned_events.idl"
    When I generate TypeScript with the independent adapter target
    Then the versioned event split keeps old names, distinct ids and one stated half

  Scenario: the Rust event split keeps the same promises
    Given a versioned REST IDL file "bdd/features/data/versioned_events_rust.idl"
    When I generate Rust with the independent adapter target
    Then the Rust versioned event split keeps the same promises
