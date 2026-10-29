@bdd_rest-versioning-apps
Feature: Version projections preserve bounded collections
  Invoice fields evolve while each version keeps the same batch limits.
  Aliases, maps, optional members, fixed arrays and bytes must compose with the
  generated projections without accepting overflow or inventing elements.

  Scenario: Rust projects bounded containers through three versions
    Given a versioned REST IDL file "bdd/features/data/versioned_bounds.idl"
    When I generate Rust with the independent adapter target
    Then the Rust bounded adapter journey preserves limits before dispatch

  Scenario: TypeScript projects the same bounded contracts
    Given a versioned REST IDL file "bdd/features/data/versioned_bounds.idl"
    When I generate TypeScript with the independent adapter target
    Then the TypeScript bounded adapter journey preserves limits before dispatch
