@bdd_rest-versioning
Feature: Imported integer masks preserve their values in Go
  A shared IDL fragment declares masks in a dependency unit.
  Its importer combines complements without changing the declared integer widths.

  Scenario: Included and imported masks compile into an executable permission policy
    Given a REST IDL file "bdd/features/data/integer_complements/api.idl"
    When I generate the split Go integer mask library
    Then the compiled Go library preserves mask bounds and permission decisions
