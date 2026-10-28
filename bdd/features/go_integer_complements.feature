@bdd_rest-versioning-core
Feature: Imported integer masks preserve their values in Go
  A shared IDL fragment declares masks in a dependency unit.
  Its importer combines complements using IDL widths across package boundaries.

  Scenario: Included and imported masks compile into an executable permission policy
    Given a REST IDL file "bdd/features/data/integer_complements/api.idl"
    When I generate the split Go integer mask library
    Then the compiled Go library preserves mask bounds and permission decisions
