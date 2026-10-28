@bdd_rest-versioning-core
Feature: Integer constants retain IDL values in generated Rust
  Constant masks share the integer rules used by fixed-array dimensions.
  Independent imports use their declared widths, and local module names cannot
  change the value of an explicitly rooted dependency reference.

  Scenario: Split constant declarations compile and preserve their values
    Given a REST IDL file "bdd/features/data/integer_constants/api.idl"
    When I generate the split integer constant Rust library
    Then the compiled constant library preserves masks and imported values
