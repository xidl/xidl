@bdd_rest-versioning
Feature: versioned transform shapes
  One profile contract exercising every change shape from the walkthrough's
  catalogue: a flat string becoming a structured route whose value set grows
  later, two fields merging into one, a precision change, a single optional
  becoming a list, a retired enum value and a flipped default. Every shape
  converts in both directions and satisfies the round-trip rules.

  Scenario: the transform shapes upcast, downcast and round-trip
    Given a versioned REST IDL file "bdd/features/data/versioned_transforms.idl"
    When I generate TypeScript with the independent adapter target
    Then the versioned transform shapes upcast, downcast and round-trip

  Scenario: the Rust transform shapes keep the same promises
    Given a versioned REST IDL file "bdd/features/data/versioned_transforms_rust.idl"
    When I generate Rust with the independent adapter target
    Then the Rust versioned transform shapes keep the same promises
